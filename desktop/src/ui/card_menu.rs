//! A card's actions on the desktop (docs/desktop-app.md §6): a "…" button that opens them as a
//! menu, and the same menu on right-click.

use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::{Selectable, Sizable};
use gpui_kit::{App, Context, ElementId, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, Window};

use crate::copy::Copy;
use crate::ui::icons::Lucide;

/// One action in a card's menu ("Renew loan", "Edit reservation", "Cancel reservation").
#[derive(Clone)]
pub struct CardAction {
    pub label: SharedString,
    pub handler: crate::ui::controls::OnClick,
}

impl CardAction {
    pub fn new(label: impl Into<SharedString>, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self { label: label.into(), handler: Rc::new(handler) }
    }
}

fn build(menu: PopupMenu, actions: &[CardAction]) -> PopupMenu {
    actions.iter().fold(menu, |menu, action| {
        let handler = action.handler.clone();
        menu.item(PopupMenuItem::new(action.label.clone()).on_click(move |_, window, cx| handler(window, cx)))
    })
}

/// The "…" button, labelled "More options", opening `actions`.
pub fn card_menu_button(id: impl Into<ElementId>, actions: Vec<CardAction>) -> impl IntoElement {
    Button::new(id)
        .ghost()
        .small()
        .icon(Lucide::Ellipsis)
        .accessibility_label(Copy::Common::moreOptions)
        .selected(false)
        .dropdown_menu(move |menu: PopupMenu, _: &mut Window, _: &mut Context<PopupMenu>| build(menu, &actions))
}

/// `element` with `actions` as its right-click menu.
pub fn with_context_menu<E>(element: E, actions: Vec<CardAction>) -> impl IntoElement
where
    E: InteractiveElement + ParentElement + Styled + IntoElement + 'static,
{
    element.context_menu(move |menu: PopupMenu, _: &mut Window, _: &mut Context<PopupMenu>| build(menu, &actions))
}
