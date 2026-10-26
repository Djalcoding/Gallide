use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Styled},
    text::{Line, ToLine},
    widgets::{
        Block,
        HighlightSpacing::{self},
        Widget,
    },
};

use crate::explore::GallideEntryVec;

pub struct EntryList<'a> {
    pub(crate) block: Option<Block<'a>>,
    /// Style used as a base style for the widget
    pub(crate) style: Style,
    /// Style used to render selected item
    pub(crate) highlight_style: Style,
    /// Symbol in front of the selected item (Shift all items to the right)
    pub(crate) highlight_symbol: Option<Line<'a>>,
    /// Whether to repeat the highlight symbol for each line of the selected item
    pub(crate) repeat_highlight_symbol: bool,
    /// Decides when to allocate spacing for the selection symbol
    pub(crate) highlight_spacing: HighlightSpacing,
    /// How many items to try to keep visible before and after the selected item
    pub(crate) scroll_padding: usize,
}

impl<'a> EntryList<'a> {
    pub fn new() -> Self {
        Self {
            block: None,
            style: Style::default(),
            highlight_style: Style::default(),
            highlight_symbol: None,
            repeat_highlight_symbol: false,
            highlight_spacing: HighlightSpacing::WhenSelected,
            scroll_padding: 10,
        }
    }

    pub fn set_block(&mut self, block: Block<'a>) {
        self.block = Some(block);
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }

    pub fn highlight_symbol<L: Into<Line<'a>>>(mut self, highlight_symbol: L) -> Self {
        self.highlight_symbol = Some(highlight_symbol.into());
        self
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn highlight_style<S: Into<Style>>(mut self, style: S) -> Self {
        self.highlight_style = style.into();
        self
    }

    pub const fn repeat_highlight_symbol(mut self, repeat: bool) -> Self {
        self.repeat_highlight_symbol = repeat;
        self
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn highlight_spacing(mut self, value: HighlightSpacing) -> Self {
        self.highlight_spacing = value;
        self
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn scroll_padding(mut self, padding: usize) -> Self {
        self.scroll_padding = padding;
        self
    }
}

impl Styled for EntryList<'_> {
    type Item = Self;

    fn style(&self) -> Style {
        self.style
    }

    fn set_style<S: Into<Style>>(self, style: S) -> Self::Item {
        self.style(style)
    }
}

impl EntryList<'_> {
    pub fn render(&self, items: &GallideEntryVec, area: Rect, buf: &mut Buffer, cursor: usize) {
        buf.set_style(area, self.style);
        self.block.as_ref().render(area, buf);
        let list_area;
        if let Some(b) = &self.block {
            list_area = b.inner(area);
        } else {
            list_area = area;
        }

        if list_area.is_empty() {
            return;
        }

        let list_height = list_area.height as usize;

        let (first_visible_index, last_visible_index) =
            self.get_items_bounds(items, Some(cursor), 0, list_height);

        // Get our set highlighted symbol (if one was set)
        let default_highlight_symbol = Line::default();
        let highlight_symbol = self
            .highlight_symbol
            .as_ref()
            .unwrap_or(&default_highlight_symbol);
        let highlight_symbol_width = highlight_symbol.width() as u16;
        let empty_symbol = " ".repeat(highlight_symbol_width as usize);
        let empty_symbol = empty_symbol.to_line();

        for (current_height, (i, item)) in (0_u16..).zip(
            items
                .into_iter()
                .enumerate()
                .skip(first_visible_index)
                .take(last_visible_index - first_visible_index),
        ) {
            let (x, y) = (list_area.left(), list_area.top() + current_height);

            let row_area = Rect::new(x, y, list_area.width, 1);

            let is_selected = i == cursor;
            let item_style = if is_selected {
                self.highlight_style
            } else {
                self.style
            };
            buf.set_style(row_area, item_style);

            let item_area = Rect {
                x: row_area.x + highlight_symbol_width,
                width: row_area.width.saturating_sub(highlight_symbol_width),
                ..row_area
            };
            Widget::render(item.name().clone(), item_area, buf);

            let line = if is_selected {
                highlight_symbol
            } else {
                &empty_symbol
            };
            let highlight_area = Rect::new(x, y, highlight_symbol_width, 1);
            line.render(highlight_area, buf);
        }
    }

    fn get_items_bounds(
        &self,
        items: &GallideEntryVec,
        selected: Option<usize>,
        offset: usize,
        max_height: usize,
    ) -> (usize, usize) {
        let offset = offset.min(items.len().saturating_sub(1));

        // Note: visible here implies visible in the given area
        let mut first_visible_index = offset;
        let mut last_visible_index = offset;

        // Current height of all items in the list to render, beginning at the offset
        let mut height_from_offset = 0;

        // Calculate the last visible index and total height of the items
        // that will fit in the available space
        for _ in items.into_iter().skip(offset) {
            if height_from_offset + 1 > max_height {
                break;
            }

            height_from_offset += 1;

            last_visible_index += 1;
        }

        // Get the selected index and apply scroll_padding to it, but still honor the offset if
        // nothing is selected. This allows for the list to stay at a position after select()ing
        // None.
        let index_to_display = self
            .apply_scroll_padding_to_selected_index(
                items,
                selected,
                max_height,
                first_visible_index,
                last_visible_index,
            )
            .unwrap_or(offset);

        // Recall that last_visible_index is the index of what we
        // can render up to in the given space after the offset
        // If we have an item selected that is out of the viewable area (or
        // the offset is still set), we still need to show this item
        while index_to_display >= last_visible_index {
            height_from_offset = height_from_offset.saturating_add(1);

            last_visible_index += 1;

            // Now we need to hide previous items since we didn't have space
            // for the selected/offset item
            while height_from_offset > max_height {
                height_from_offset = height_from_offset.saturating_sub(1);

                // Remove this item to view by starting at the next item index
                first_visible_index += 1;
            }
        }

        // Here we're doing something similar to what we just did above
        // If the selected item index is not in the viewable area, let's try to show the item
        while index_to_display < first_visible_index {
            first_visible_index -= 1;

            height_from_offset = height_from_offset.saturating_add(1);

            // Don't show an item if it is beyond our viewable height
            while height_from_offset > max_height {
                last_visible_index -= 1;

                height_from_offset = height_from_offset.saturating_sub(1);
            }
        }

        (first_visible_index, last_visible_index)
    }

    /// Applies scroll padding to the selected index, reducing the padding value to keep the
    /// selected item on screen even with items of inconsistent sizes
    ///
    /// This function is sensitive to how the bounds checking function handles item height
    fn apply_scroll_padding_to_selected_index(
        &self,
        items: &GallideEntryVec,
        selected: Option<usize>,
        max_height: usize,
        first_visible_index: usize,
        last_visible_index: usize,
    ) -> Option<usize> {
        let last_valid_index = items.len().saturating_sub(1);
        let selected = selected?.min(last_valid_index);

        // The below loop handles situations where the list item sizes may not be consistent,
        // where the offset would have excluded some items that we want to include, or could
        // cause the offset value to be set to an inconsistent value each time we render.
        // The padding value will be reduced in case any of these issues would occur
        let mut scroll_padding = self.scroll_padding;
        while scroll_padding > 0 {
            let mut height_around_selected = 0;
            for _ in selected.saturating_sub(scroll_padding)
                ..=selected
                    .saturating_add(scroll_padding)
                    .min(last_valid_index)
            {
                height_around_selected += 1;
            }
            if height_around_selected <= max_height {
                break;
            }
            scroll_padding -= 1;
        }

        Some(
            if (selected + scroll_padding).min(last_valid_index) >= last_visible_index {
                selected + scroll_padding
            } else if selected.saturating_sub(scroll_padding) < first_visible_index {
                selected.saturating_sub(scroll_padding)
            } else {
                selected
            }
            .min(last_valid_index),
        )
    }
}

impl Default for  EntryList<'_>{
    fn default() -> Self {
        EntryList::new()
    } 
}
