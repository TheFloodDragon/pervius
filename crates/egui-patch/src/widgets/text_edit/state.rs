use std::sync::Arc;

use crate::mutex::Mutex;

use crate::{
    text_selection::{CCursorRange, TextCursorState},
    Context, Galley, Id, Vec2,
};

pub type TextEditUndoer = crate::util::undoer::Undoer<(CCursorRange, String)>;

/// The text edit state stored between frames.
///
/// Attention: You also need to `store` the updated state.
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut text = String::new();
/// use egui::text::{CCursor, CCursorRange};
///
/// let mut output = egui::TextEdit::singleline(&mut text).show(ui);
///
/// // Create a new selection range
/// let min = CCursor::new(0);
/// let max = CCursor::new(0);
/// let new_range = CCursorRange::two(min, max);
///
/// // Update the state
/// output.state.cursor.set_char_range(Some(new_range));
/// // Store the updated state
/// output.state.store(ui.ctx(), output.response.id);
/// # });
/// ```
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextEditState {
    /// Controls the text selection.
    pub cursor: TextCursorState,

    /// Wrapped in Arc for cheaper clones.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) undoer: Arc<Mutex<TextEditUndoer>>,

    // If IME candidate window is shown on this text edit.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) ime_enabled: bool,

    // cursor range for IME candidate.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) ime_cursor_range: CCursorRange,

    // Text offset within the widget area.
    // Used for sensing and singleline text clipping.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) text_offset: Vec2,

    /// When did the user last press a key or click on the `TextEdit`.
    /// Used to pause the cursor animation when typing.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) last_interaction_time: f64,

    /// Cached galley with selection rectangles painted in.
    /// Avoids re-cloning large galleys every frame when cursor hasn't moved.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) selection_cache: Option<SelectionCache>,

    /// Cached galley with large rows split for tessellation culling.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) split_galley_cache: Option<SplitGalleyCache>,
}

/// Cached result of `paint_text_selection` to avoid per-frame deep-clone of large galleys.
#[derive(Clone)]
pub(crate) struct SelectionCache {
    pub cursor_range: CCursorRange,
    /// Pointer identity of the source (unmodified) galley, used to detect galley changes.
    pub source_galley_addr: usize,
    /// The galley with selection rectangles baked in.
    pub painted_galley: Arc<Galley>,
}

/// Cached galley with large rows split into smaller chunks for tessellation culling.
#[derive(Clone)]
pub(crate) struct SplitGalleyCache {
    /// Pointer identity of the input galley (after selection painting).
    pub source_addr: usize,
    /// The galley with large rows split into chunks.
    pub split_galley: Arc<Galley>,
}

impl TextEditState {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }

    pub fn undoer(&self) -> TextEditUndoer {
        self.undoer.lock().clone()
    }

    #[expect(clippy::needless_pass_by_ref_mut)] // Intentionally hide interiority of mutability
    pub fn set_undoer(&mut self, undoer: TextEditUndoer) {
        *self.undoer.lock() = undoer;
    }

    pub fn clear_undoer(&mut self) {
        self.set_undoer(TextEditUndoer::default());
    }
}
