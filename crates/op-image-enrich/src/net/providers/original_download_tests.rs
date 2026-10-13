use super::*;

fn png_bytes(image: &image::RgbaImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn delivered_image(url: &str) -> image::DynamicImage {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    let bytes = B64.decode(url.split_once(',').unwrap().1).unwrap();
    assert!(bytes.len() <= MAX_EMBEDDED_IMAGE_BYTES);
    image::load_from_memory(&bytes).unwrap()
}

#[test]
fn highly_compressed_original_must_still_obey_delivery_dimensions() {
    let source = image::RgbaImage::from_pixel(4096, 4, image::Rgba([255, 255, 255, 255]));
    let png = png_bytes(&source);
    assert!(
        crate::net::downscale::maybe_downscale(&png).is_none(),
        "fixture must exercise the byte-savings refusal"
    );
    let url = original_data_url("image/png", &png).expect("resized original");
    let output = delivered_image(&url);
    assert_eq!((output.width(), output.height()), (2048, 2));
}

#[test]
fn resized_original_preserves_transparency() {
    let source = image::RgbaImage::from_fn(4096, 8, |x, _| {
        image::Rgba([90, 120, 150, if x < 2048 { 0 } else { 128 }])
    });
    let url = original_data_url("image/png", &png_bytes(&source)).expect("resized alpha image");
    assert!(url.starts_with("data:image/png;base64,"));
    let output = delivered_image(&url).to_rgba8();
    assert_eq!(output.dimensions(), (2048, 4));
    assert_eq!(output.get_pixel(100, 1).0[3], 0);
    assert!((127..=129).contains(&output.get_pixel(1900, 1).0[3]));
}

#[test]
fn original_dimensions_bound_decode_allocation_before_conversion() {
    assert!(safe_dimensions(8000, 8000));
    assert!(safe_dimensions(32_768, 100));
    assert!(!safe_dimensions(0, 800));
    assert!(!safe_dimensions(800, 0));
    assert!(!safe_dimensions(8001, 8000));
    assert!(!safe_dimensions(32_769, 1));
    assert!(!safe_dimensions(u32::MAX, u32::MAX));
}

#[test]
fn original_url_rejects_unsafe_destinations_before_resolving() {
    for url in [
        "http://upload.wikimedia.org/photo.jpg",
        "file:///etc/passwd",
        "https://user:password@upload.wikimedia.org/photo.jpg",
        "https://localhost/photo.jpg",
        "https://a.localhost/photo.jpg",
        "https://machine.local/photo.jpg",
        "https://metadata.google.internal/photo.jpg",
        "https://localhost./photo.jpg",
        "https://example.com/photo.jpg",
        "https://127.0.0.1/photo.jpg",
        "https://2130706433/photo.jpg",
        "https://0x7f000001/photo.jpg",
        "https://[::1]/photo.jpg",
        "https://[::ffff:127.0.0.1]/photo.jpg",
        "https://192.168.1.1/photo.jpg",
        "https://169.254.169.254/photo.jpg",
        "https://198.18.0.1/photo.jpg",
        "https://[2002:7f00:1::]/photo.jpg",
    ] {
        assert!(safe_original_url(url).is_none(), "accepted {url}");
    }
    let url = safe_original_url("https://upload.wikimedia.org/a.jpg?width=2048#preview")
        .expect("public HTTPS original");
    assert_eq!(
        url.as_str(),
        "https://upload.wikimedia.org/a.jpg?width=2048"
    );
}

#[test]
fn all_dns_answers_must_be_public() {
    let public: SocketAddr = "93.184.215.14:443".parse().unwrap();
    assert!(validate_addresses(vec![public]).is_some());
    assert!(validate_addresses(vec![]).is_none());
    for ip in [
        "0.0.0.0",
        "10.1.2.3",
        "100.64.0.1",
        "127.0.0.2",
        "169.254.1.2",
        "172.31.2.3",
        "192.0.0.1",
        "192.0.2.1",
        "192.88.99.1",
        "192.168.0.1",
        "198.19.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "fc00::1",
        "fe80::1",
        "ff02::1",
        "2001:db8::1",
        "2001:2::1",
        "3fff::1",
    ] {
        let address = SocketAddr::new(ip.parse().unwrap(), 443);
        assert!(
            validate_addresses(vec![public, address]).is_none(),
            "accepted {ip}"
        );
    }
    assert!(public_ip("2606:4700:4700::1111".parse().unwrap()));
}

#[test]
fn redirects_revalidate_targets_and_stop_at_the_shared_hop_limit() {
    let url = safe_original_url("https://upload.wikimedia.org/image.jpg").unwrap();
    assert_eq!(
        checked_redirect(&url, "/original.jpg", 0).unwrap().path(),
        "/original.jpg"
    );
    for location in [
        "https://localhost/secret",
        "//127.0.0.1/secret",
        "http://upload.wikimedia.org/a.jpg",
        "https://name:secret@images.unsplash.com/a.jpg",
    ] {
        assert!(checked_redirect(&url, location, 0).is_none());
    }
    assert!(checked_redirect(&url, "/last.jpg", MAX_REDIRECTS - 1).is_some());
    assert!(checked_redirect(&url, "/loop.jpg", MAX_REDIRECTS).is_none());
}

#[test]
fn original_opportunity_never_extends_the_caller_deadline() {
    let now = Instant::now();
    assert_eq!(
        opportunity_deadline(now, None),
        Some(now + ORIGINAL_FETCH_BUDGET)
    );
    let sooner = now + Duration::from_millis(25);
    assert_eq!(opportunity_deadline(now, Some(sooner)), Some(sooner));
    assert_eq!(
        opportunity_deadline(now, Some(now + Duration::from_secs(60))),
        Some(now + ORIGINAL_FETCH_BUDGET)
    );
    assert_eq!(opportunity_deadline(now, Some(now)), None);
    let result = crate::net::block_on_image_runtime(fetch_original_image_data_url(
        "https://upload.wikimedia.org/image.jpg",
        Some(now),
    ));
    assert!(result.is_none());
}

#[test]
fn original_reader_aborts_both_declared_and_chunked_oversize_bodies() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    for response in [
        "HTTP/1.1 200 OK\r\nContent-Length: 17\r\nConnection: close\r\n\r\n0123456789abcdefg",
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n11\r\n0123456789abcdefg\r\n0\r\n\r\n",
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut request = [0; 1024];
            assert!(stream.read(&mut request).unwrap() > 0);
            stream.write_all(response.as_bytes()).unwrap();
        });
        let bytes = crate::net::block_on_image_runtime(async {
            let response = reqwest::Client::builder().no_proxy().build().unwrap()
                .get(format!("http://{address}"))
                .timeout(Duration::from_secs(2)).send().await.unwrap();
            read_capped(response, 16).await
        });
        assert!(bytes.is_none());
        server.join().unwrap();
    }
    assert_eq!(MAX_EMBEDDED_IMAGE_BYTES, 4 * 1024 * 1024);
}
