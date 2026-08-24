//! Reusable, open-once PDF page rendering for Rust callers.
//!
//! [`PdfRenderSession`] keeps Xberg's parsed PDF document alive across multiple
//! page renders. Applications that need more than one page can therefore avoid
//! reopening and reparsing the same bytes for every call while continuing to use
//! Xberg's existing rendering safeguards and diagnostics.

use super::render::{document_page_count, open_pdf_document, render_open_pdf_page_to_png};
use crate::Result;
use crate::error::XbergError;

/// A reusable rendering session for one PDF document.
///
/// Opening a PDF parses its cross-reference table, trailer, page tree, and other
/// document-level structures. Keeping the session alive lets callers render
/// multiple pages from that parsed document instead of repeating that work for
/// every page.
///
/// The native PDF handle is deliberately private. This keeps Xberg's rendering
/// backend an implementation detail while exposing the efficient document
/// lifetime needed by viewers, OCR pipelines, preview services, thumbnail
/// generators, and other page-oriented applications.
///
/// Rendering delegates to the same path as
/// [`crate::pdf::render::render_pdf_page_to_png`], including the extreme-page
/// dimension safeguard, panic containment, and opt-in render diagnostics.
///
/// # Example
///
/// ```no_run
/// use xberg::pdf::PdfRenderSession;
///
/// # fn render_document(pdf_bytes: &[u8]) -> xberg::Result<()> {
/// let session = PdfRenderSession::open(pdf_bytes, None)?;
///
/// for page_index in 0..session.page_count() {
///     let png = session.render_page_to_png(page_index, Some(150))?;
///     // Consume or persist `png` before rendering the next page.
///     # let _ = png;
/// }
/// # Ok(())
/// # }
/// ```
#[cfg_attr(alef, alef(skip))]
pub struct PdfRenderSession {
    document: xberg_native_pdf::PdfDocument,
    page_count: usize,
}

impl PdfRenderSession {
    /// Open and parse a PDF document for repeated page rendering.
    ///
    /// `password` is used to authenticate encrypted PDFs before the session is
    /// returned. The input bytes are copied into the owned native document, so
    /// the caller does not need to retain the original byte slice afterward.
    ///
    /// # Errors
    ///
    /// Returns [`XbergError::Parsing`] if the PDF cannot be opened,
    /// authenticated, or its page count cannot be read.
    pub fn open(pdf_bytes: &[u8], password: Option<&str>) -> Result<Self> {
        let document = open_pdf_document(pdf_bytes, password)?;
        let page_count = document_page_count(&document)?;

        Ok(Self { document, page_count })
    }

    /// Return the number of pages available in this document.
    #[inline]
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Render one zero-based page index to PNG-encoded bytes.
    ///
    /// `dpi` defaults to 150. Values below 1 are clamped to 1, matching
    /// [`crate::pdf::render::render_pdf_page_to_png`]. Pages with extreme
    /// dimensions may be rendered at a lower effective DPI by Xberg's existing
    /// safety guard.
    ///
    /// # Errors
    ///
    /// Returns [`XbergError::Parsing`] if `page_index` is out of range or the
    /// selected page cannot be rendered.
    pub fn render_page_to_png(&self, page_index: usize, dpi: Option<i32>) -> Result<Vec<u8>> {
        if page_index >= self.page_count {
            return Err(XbergError::Parsing {
                message: format!(
                    "Page index {page_index} out of range (document has {} pages)",
                    self.page_count
                ),
                source: None,
            });
        }

        render_open_pdf_page_to_png(&self.document, page_index, dpi)
    }
}

impl std::fmt::Debug for PdfRenderSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PdfRenderSession")
            .field("page_count", &self.page_count)
            .finish_non_exhaustive()
    }
}
