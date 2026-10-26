//! Every stack's rule against its fixture. Webflow and Shopify use the Worker.

use khalid_core::AbsoluteUrl;
use khalid_mount::{MountError, MountPath, Stack, proxy_rule};

#[cfg(test)]
fn rule(stack: Stack, mount: &str, blog: &str) -> Result<String, MountError> {
    let blog = AbsoluteUrl::parse(blog).unwrap();
    proxy_rule(stack, &MountPath::try_new(mount)?, &blog)
}

#[test]
fn every_stack_prints_its_fixture_byte_for_byte() {
    for stack in Stack::ALL {
        let name = match stack {
            Stack::Webflow | Stack::Shopify => "cloudflare-worker",
            other => other.name(),
        };
        let fixture = format!("{}/fixtures/{name}.txt", env!("CARGO_MANIFEST_DIR"));
        let want = std::fs::read_to_string(fixture).unwrap();
        let got = rule(stack, "/blog", "https://blog-origin.example.com").unwrap();
        assert_eq!(got, want, "stack {stack}");
    }
}

#[test]
fn a_nested_mount_and_a_port_reach_the_snippet() {
    let got = rule(Stack::Apache, "/resources/blog", "http://10.0.0.5:8080/").unwrap();
    assert!(got.contains("\nProxyPass /resources/blog/ http://10.0.0.5:8080/resources/blog/\n"));
}

#[test]
fn stack_names_parse_back_and_an_unknown_one_lists_them_all() {
    for stack in Stack::ALL {
        assert_eq!(stack.to_string().parse::<Stack>(), Ok(stack));
    }
    assert_eq!("Next.js".parse::<Stack>(), Ok(Stack::Nextjs));
    let error = "wordpress".parse::<Stack>().unwrap_err().to_string();
    assert!(error.starts_with("no proxy rule for `wordpress`; the stacks are nginx, caddy"));
}

#[test]
fn mount_paths_that_cannot_be_a_folder_are_refused() {
    for (text, want) in [
        ("/", MountError::Root),
        ("blog", MountError::Shape),
        ("/blog/", MountError::Shape),
        ("/a//b", MountError::Shape),
        ("/a/../b", MountError::Shape),
        ("/blog\";", MountError::Character { character: '"' }),
        ("/{x}", MountError::Character { character: '{' }),
    ] {
        assert_eq!(MountPath::try_new(text), Err(want), "mount {text:?}");
    }
    assert!(MountPath::try_new("/Docs_v2/blog-1.0~x").is_ok());
}

#[test]
fn a_blog_address_with_a_path_or_query_is_refused() {
    for blog in ["https://b.example.com/khalid", "https://b.example.com/?a=1"] {
        let got = rule(Stack::Nginx, "/blog", blog);
        assert_eq!(got, Err(MountError::UpstreamPath), "address {blog}");
    }
}
