//! The page addresses a missing address may be sent to (KH-SEO-8).

use khalid_api::redirect_desk::{PageAddresses, StaticPages};
use khalid_seo::UrlPath;

#[tokio::test]
async fn static_pages_list_their_addresses_in_order() {
    let pages = StaticPages::new(&["/blog/", "/blog/robots-txt/"]).unwrap();
    let listed = pages.addresses().await.unwrap();
    let listed: Vec<&str> = listed.iter().map(UrlPath::as_str).collect();
    assert_eq!(listed, ["/blog/", "/blog/robots-txt/"]);
    assert!(StaticPages::new(&["blog/no-slash/"]).is_err());
}
