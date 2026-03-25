use crate::error::Error;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// A page of results from a paginated API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResponse<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub total_count: u32,
}

impl<T> PagedResponse<T> {
    /// Returns true if there are more pages after this one.
    pub fn has_next_page(&self) -> bool {
        self.page < self.total_pages
    }
}

/// Convert an `ApiListResponse` into a `PagedResponse`.
impl<T> From<(Vec<T>, Option<crate::models::PaginationMeta>)> for PagedResponse<T> {
    fn from((data, meta): (Vec<T>, Option<crate::models::PaginationMeta>)) -> Self {
        let meta = meta.unwrap_or(crate::models::PaginationMeta {
            page: 1,
            per: data.len() as u32,
            total: data.len() as u32,
            total_pages: 1,
        });
        // Compute total_pages from total/per if the API didn't provide it
        let total_pages = if meta.total_pages > 0 {
            meta.total_pages
        } else if meta.per > 0 {
            meta.total.div_ceil(meta.per)
        } else {
            1
        };
        PagedResponse {
            data,
            page: meta.page,
            per_page: meta.per,
            total_pages: total_pages.max(1),
            total_count: meta.total,
        }
    }
}

/// Collect all pages from a paginated fetch function into a single `Vec<T>`.
///
/// `fetch` takes `(page, per_page)` and returns a `PagedResponse<T>`.
pub async fn collect_all_pages<T, Fut, F>(per_page: u32, fetch: F) -> Result<Vec<T>, Error>
where
    F: Fn(u32, u32) -> Fut,
    Fut: Future<Output = Result<PagedResponse<T>, Error>>,
{
    let mut all = Vec::new();
    let mut page = 1u32;
    loop {
        let paged = fetch(page, per_page).await?;
        let has_next = paged.has_next_page();
        all.extend(paged.data);
        if !has_next {
            break;
        }
        page += 1;
    }
    Ok(all)
}

/// Stream all items from a paginated fetch function.
///
/// Items are yielded one at a time as they arrive from each page.
pub fn stream_all_pages<T, Fut, F>(
    per_page: u32,
    fetch: F,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>
where
    T: Send + 'static,
    F: Fn(u32, u32) -> Fut + Send + 'static,
    Fut: Future<Output = Result<PagedResponse<T>, Error>> + Send + 'static,
{
    Box::pin(async_stream::try_stream! {
        let mut page = 1u32;
        loop {
            let paged = fetch(page, per_page).await?;
            let has_next = paged.has_next_page();
            for item in paged.data {
                yield item;
            }
            if !has_next {
                break;
            }
            page += 1;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PaginationMeta;
    use futures::StreamExt;

    fn make_meta(page: u32, per: u32, total: u32, total_pages: u32) -> PaginationMeta {
        PaginationMeta {
            page,
            per,
            total,
            total_pages,
        }
    }

    #[test]
    fn paged_response_has_next_page() {
        let paged: PagedResponse<i32> = PagedResponse {
            data: vec![1, 2, 3],
            page: 1,
            per_page: 3,
            total_pages: 3,
            total_count: 9,
        };
        assert!(paged.has_next_page());
    }

    #[test]
    fn paged_response_last_page() {
        let paged: PagedResponse<i32> = PagedResponse {
            data: vec![7, 8, 9],
            page: 3,
            per_page: 3,
            total_pages: 3,
            total_count: 9,
        };
        assert!(!paged.has_next_page());
    }

    #[test]
    fn from_meta_conversion() {
        let meta = make_meta(2, 10, 50, 5);
        let paged: PagedResponse<String> = (vec!["a".to_string()], Some(meta)).into();
        assert_eq!(paged.page, 2);
        assert_eq!(paged.per_page, 10);
        assert_eq!(paged.total_pages, 5);
        assert_eq!(paged.total_count, 50);
    }

    #[test]
    fn from_no_meta() {
        let data: Vec<i32> = vec![1, 2, 3];
        let paged: PagedResponse<i32> = (data, None).into();
        assert_eq!(paged.page, 1);
        assert_eq!(paged.total_pages, 1);
        assert!(!paged.has_next_page());
    }

    #[tokio::test]
    async fn collect_all_pages_single_page() {
        let result = collect_all_pages::<i32, _, _>(10, |page, _per| async move {
            assert_eq!(page, 1);
            Ok(PagedResponse {
                data: vec![1, 2, 3],
                page: 1,
                per_page: 10,
                total_pages: 1,
                total_count: 3,
            })
        })
        .await
        .unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn collect_all_pages_multiple_pages() {
        let result = collect_all_pages::<i32, _, _>(2, |page, _per| async move {
            match page {
                1 => Ok(PagedResponse {
                    data: vec![1, 2],
                    page: 1,
                    per_page: 2,
                    total_pages: 3,
                    total_count: 6,
                }),
                2 => Ok(PagedResponse {
                    data: vec![3, 4],
                    page: 2,
                    per_page: 2,
                    total_pages: 3,
                    total_count: 6,
                }),
                _ => Ok(PagedResponse {
                    data: vec![5, 6],
                    page: 3,
                    per_page: 2,
                    total_pages: 3,
                    total_count: 6,
                }),
            }
        })
        .await
        .unwrap();
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[tokio::test]
    async fn collect_all_pages_error_propagates() {
        let result = collect_all_pages::<i32, _, _>(10, |_page, _per| async {
            Err::<PagedResponse<i32>, Error>(Error::Auth("nope".into()))
        })
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stream_all_pages_yields_all() {
        let stream = stream_all_pages::<i32, _, _>(2, |page, _per| async move {
            match page {
                1 => Ok(PagedResponse {
                    data: vec![10, 20],
                    page: 1,
                    per_page: 2,
                    total_pages: 2,
                    total_count: 4,
                }),
                _ => Ok(PagedResponse {
                    data: vec![30, 40],
                    page: 2,
                    per_page: 2,
                    total_pages: 2,
                    total_count: 4,
                }),
            }
        });

        let items: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
        assert_eq!(items, vec![10, 20, 30, 40]);
    }
}
