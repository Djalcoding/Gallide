use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Styled},
    text::{Line, ToLine},
    widgets::{
        Block,
        HighlightSpacing::{self, Always, Never, WhenSelected},
        List, ListItem, ListState, StatefulWidget, Widget,
    },
};

use crate::explore::GallideEntryVec;

pub struct EntryList<'a> {
    pub(crate) block: Option<Block<'a>>,
    /// Style used as a base style for the widget
    pub(crate) style: Style,
    /// List display direction
    pub(crate) direction: ListDirection,
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

/// Defines the direction in which the list will be rendered.
///
/// If there are too few items to fill the screen, the list will stick to the starting edge.
///
/// See [`List::direction`].
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ListDirection {
    /// The first value is on the top, going to the bottom
    #[default]
    TopToBottom,
    /// The first value is on the bottom, going to the top.
    BottomToTop,
}

impl<'a> EntryList<'a> {
    /// Creates a new list from [`ListItem`]s
    ///
    /// The `items` parameter accepts any value that can be converted into an iterator of
    /// [`Into<ListItem>`]. This includes arrays of [`&str`] or [`Vec`]s of [`Text`].
    ///
    /// # Example
    ///
    /// From a slice of [`&str`]
    ///
    /// ```
    /// use ratatui::widgets::List;
    ///
    /// let list = List::new(["Item 1", "Item 2"]);
    /// ```
    ///
    /// From [`Text`]
    ///
    /// ```
    /// use ratatui::style::{Style, Stylize};
    /// use ratatui::text::Text;
    /// use ratatui::widgets::List;
    ///
    /// let list = List::new([
    ///     Text::styled("Item 1", Style::new().red()),
    ///     Text::styled("Item 2", Style::new().red()),
    /// ]);
    /// ```
    ///
    /// You can also create an empty list using the [`Default`] implementation and use the
    /// [`List::items`] fluent setter.
    ///
    /// ```rust
    /// use ratatui::widgets::List;
    ///
    /// let empty_list = List::default();
    /// let filled_list = empty_list.items(["Item 1"]);
    /// ```
    ///
    /// [`Text`]: ratatui_core::text::Text
    pub fn new() -> Self {
        Self {
            block: None,
            style: Style::default(),
            direction: ListDirection::default(),
            highlight_style: Style::default(),
            highlight_symbol: None,
            repeat_highlight_symbol: false,
            highlight_spacing: HighlightSpacing::WhenSelected,
            scroll_padding: 10,
        }
    }

    /// Wraps the list with a custom [`Block`] widget.
    ///
    /// The `block` parameter holds the specified [`Block`] to be created around the [`List`]
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::widgets::{Block, List};
    ///
    /// let items = ["Item 1"];
    /// let block = Block::bordered().title("List");
    /// let list = List::new(items).block(block);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the base style of the widget
    ///
    /// `style` accepts any type that is convertible to [`Style`] (e.g. [`Style`], [`Color`], or
    /// your own type that implements [`Into<Style>`]).
    ///
    /// All text rendered by the widget will use this style, unless overridden by [`Block::style`],
    /// [`ListItem::style`], or the styles of the [`ListItem`]'s content.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::style::{Style, Stylize};
    /// use ratatui::widgets::List;
    ///
    /// let items = ["Item 1"];
    /// let list = List::new(items).style(Style::new().red().italic());
    /// ```
    ///
    /// `List` also implements the [`Styled`] trait, which means you can use style shorthands from
    /// the [`Stylize`] trait to set the style of the widget more concisely.
    ///
    /// [`Stylize`]: ratatui_core::style::Stylize
    ///
    /// ```rust
    /// use ratatui::style::Stylize;
    /// use ratatui::widgets::List;
    ///
    /// let items = ["Item 1"];
    /// let list = List::new(items).red().italic();
    /// ```
    ///
    /// [`Color`]: ratatui_core::style::Color
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }

    /// Set the symbol to be displayed in front of the selected item
    ///
    /// By default there are no highlight symbol.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::widgets::List;
    ///
    /// let items = ["Item 1", "Item 2"];
    /// let list = List::new(items).highlight_symbol(">>");
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn highlight_symbol<L: Into<Line<'a>>>(mut self, highlight_symbol: L) -> Self {
        self.highlight_symbol = Some(highlight_symbol.into());
        self
    }

    /// Set the style of the selected item
    ///
    /// `style` accepts any type that is convertible to [`Style`] (e.g. [`Style`], [`Color`], or
    /// your own type that implements [`Into<Style>`]).
    ///
    /// This style will be applied to the entire item, including the
    /// [highlight symbol](List::highlight_symbol) if it is displayed, and will override any style
    /// set on the item or on the individual cells.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::style::{Style, Stylize};
    /// use ratatui::widgets::List;
    ///
    /// let items = ["Item 1", "Item 2"];
    /// let list = List::new(items).highlight_style(Style::new().red().italic());
    /// ```
    ///
    /// [`Color`]: ratatui_core::style::Color
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn highlight_style<S: Into<Style>>(mut self, style: S) -> Self {
        self.highlight_style = style.into();
        self
    }

    /// Set whether to repeat the highlight symbol and style over selected multi-line items
    ///
    /// This is `false` by default.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn repeat_highlight_symbol(mut self, repeat: bool) -> Self {
        self.repeat_highlight_symbol = repeat;
        self
    }

    /// Set when to show the highlight spacing
    ///
    /// The highlight spacing is the spacing that is allocated for the selection symbol (if enabled)
    /// and is used to shift the list when an item is selected. This method allows you to configure
    /// when this spacing is allocated.
    ///
    /// - [`HighlightSpacing::Always`] will always allocate the spacing, regardless of whether an
    ///   item is selected or not. This means that the table will never change size, regardless of
    ///   if an item is selected or not.
    /// - [`HighlightSpacing::WhenSelected`] will only allocate the spacing if an item is selected.
    ///   This means that the table will shift when an item is selected. This is the default setting
    ///   for backwards compatibility, but it is recommended to use `HighlightSpacing::Always` for a
    ///   better user experience.
    /// - [`HighlightSpacing::Never`] will never allocate the spacing, regardless of whether an item
    ///   is selected or not. This means that the highlight symbol will never be drawn.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ratatui::widgets::{HighlightSpacing, List};
    ///
    /// let items = ["Item 1"];
    /// let list = List::new(items).highlight_spacing(HighlightSpacing::Always);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn highlight_spacing(mut self, value: HighlightSpacing) -> Self {
        self.highlight_spacing = value;
        self
    }

    /// Defines the list direction (up or down)
    ///
    /// Defines if the `List` is displayed *top to bottom* (default) or *bottom to top*.
    /// If there is too few items to fill the screen, the list will stick to the starting edge.
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Example
    ///
    /// Bottom to top
    ///
    /// ```rust
    /// use ratatui::widgets::{List, ListDirection};
    ///
    /// let items = ["Item 1"];
    /// let list = List::new(items).direction(ListDirection::BottomToTop);
    /// ```
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn direction(mut self, direction: ListDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the number of items around the currently selected item that should be kept visible
    ///
    /// This is a fluent setter method which must be chained or used as it consumes self
    ///
    /// # Example
    ///
    /// A padding value of 1 will keep 1 item above and 1 item below visible if possible
    ///
    /// ```rust
    /// use ratatui::widgets::List;
    ///
    /// let items = ["Item 1"];
    /// let list = List::new(items).scroll_padding(1);
    /// ```
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
    pub fn render(
        &self,
        items: &GallideEntryVec,
        area: Rect,
        buf: &mut Buffer,
        state: &mut ListState,
    ) {
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

        if items.is_empty() {
            state.select(None);
            return;
        }

        let list_height = list_area.height as usize;

        let (first_visible_index, last_visible_index) =
            self.get_items_bounds(items, state.selected(), state.offset(), list_height);

        // Important: this changes the state's offset to be the beginning of the now viewable items
        *state.offset_mut() = first_visible_index;

        // Get our set highlighted symbol (if one was set)
        let default_highlight_symbol = Line::default();
        let highlight_symbol = self
            .highlight_symbol
            .as_ref()
            .unwrap_or(&default_highlight_symbol);
        let highlight_symbol_width = highlight_symbol.width() as u16;
        let empty_symbol = " ".repeat(highlight_symbol_width as usize);
        let empty_symbol = empty_symbol.to_line();

        let mut current_height = 0;
        let selection_spacing: bool = match self.highlight_spacing {
            Always => true,
            WhenSelected => state.selected().is_some(),
            Never => false,
        };
        for (i, item) in items
            .iter()
            .enumerate()
            .skip(state.offset())
            .take(last_visible_index - first_visible_index)
        {
            let (x, y) = if self.direction == ListDirection::BottomToTop {
                current_height += 1u16;
                (list_area.left(), list_area.bottom() - current_height)
            } else {
                let pos = (list_area.left(), list_area.top() + current_height);
                current_height += 1u16;
                pos
            };

            let row_area = Rect::new(x, y, list_area.width, 1u16);

            let item_style = self.style;
            buf.set_style(row_area, item_style);

            let is_selected = state.selected() == Some(i);

            let item_area = if selection_spacing {
                Rect {
                    x: row_area.x + highlight_symbol_width,
                    width: row_area.width.saturating_sub(highlight_symbol_width),
                    ..row_area
                }
            } else {
                row_area
            };
            Widget::render(item.name().clone(), item_area, buf);

            if is_selected {
                buf.set_style(row_area, self.highlight_style);
            }
            if selection_spacing {
                for j in 0..1 {
                    // if the item is selected, we need to display the highlight symbol:
                    // - either for the first line of the item only,
                    // - or for each line of the item if the appropriate option is set
                    let line = if is_selected && (j == 0 || self.repeat_highlight_symbol) {
                        highlight_symbol
                    } else {
                        &empty_symbol
                    };
                    let highlight_area = Rect::new(x, y + j as u16, highlight_symbol_width, 1);
                    line.render(highlight_area, buf);
                }
            }
        }
    }
}

impl EntryList<'_> {
    /// Given an offset, calculate which items can fit in a given area
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
        for item in items.iter().skip(offset) {
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
            for index in selected.saturating_sub(scroll_padding)
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
