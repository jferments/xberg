#![cfg(feature = "pdf")]

use lopdf::{Document, Object, Stream, dictionary};
use xberg::XbergError;
use xberg::pdf::{PdfRenderSession, render::render_pdf_page_to_png};

fn build_two_page_pdf() -> Vec<u8> {
    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let page_one_id = document.new_object_id();
    let page_two_id = document.new_object_id();
    let empty_content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));

    document.objects.insert(
        page_one_id,
        Object::Dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => dictionary! {},
            "Contents" => empty_content_id,
        }),
    );
    document.objects.insert(
        page_two_id,
        Object::Dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 100.into(), 200.into()],
            "Resources" => dictionary! {},
            "Contents" => empty_content_id,
        }),
    );
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_one_id.into(), page_two_id.into()],
            "Count" => 2,
        }),
    );

    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document
        .save_to(&mut bytes)
        .expect("two-page PDF fixture should serialize");
    bytes
}

#[test]
fn session_reports_page_count_and_renders_multiple_pages() {
    let pdf = build_two_page_pdf();
    let session = PdfRenderSession::open(&pdf, None).expect("valid PDF should open");

    assert_eq!(session.page_count(), 2);

    let first_png = session
        .render_page_to_png(0, Some(72))
        .expect("first page should render");
    let second_png = session
        .render_page_to_png(1, Some(72))
        .expect("second page should render");

    assert!(first_png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(second_png.starts_with(b"\x89PNG\r\n\x1a\n"));

    let first_image = image::load_from_memory(&first_png).expect("first render should decode as PNG");
    let second_image = image::load_from_memory(&second_png).expect("second render should decode as PNG");

    assert!(
        first_image.width() > first_image.height(),
        "the first page fixture is landscape"
    );
    assert!(
        second_image.height() > second_image.width(),
        "the second page fixture is portrait"
    );
}

#[test]
fn session_rejects_out_of_range_page_before_rendering() {
    let pdf = build_two_page_pdf();
    let session = PdfRenderSession::open(&pdf, None).expect("valid PDF should open");

    let error = session
        .render_page_to_png(2, Some(72))
        .expect_err("page index equal to page count must be rejected");

    match error {
        XbergError::Parsing { message, .. } => assert_eq!(message, "Page index 2 out of range (document has 2 pages)"),
        other => panic!("expected a Parsing error, got {other:?}"),
    }
}

#[test]
fn session_open_rejects_invalid_pdf_bytes() {
    let error = match PdfRenderSession::open(b"not a pdf", None) {
        Ok(_) => panic!("invalid PDF bytes must not create a session"),
        Err(error) => error,
    };

    assert!(
        matches!(error, XbergError::Parsing { .. }),
        "expected a Parsing error, got {error:?}"
    );
}

#[test]
fn session_render_matches_existing_single_page_api() {
    let pdf = build_two_page_pdf();
    let session = PdfRenderSession::open(&pdf, None).expect("valid PDF should open");

    let from_session = session
        .render_page_to_png(0, Some(72))
        .expect("session render should succeed");
    let from_convenience =
        render_pdf_page_to_png(&pdf, 0, Some(72), None).expect("existing convenience render should succeed");

    assert_eq!(
        from_session, from_convenience,
        "the session must delegate to the established rendering path"
    );
}
