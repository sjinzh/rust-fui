use fui_core::{ControlObject, Property, Style, ViewContext};
use fui_macros::ui;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use typed_builder::TypedBuilder;
use typemap::TypeMap;

use crate::controls::*;
use crate::layout::*;
use crate::{DataHolder, GestureArea};
use fui_core::*;

pub enum MenuItem {
    Separator,
    Text {
        text: String,
        shortcut: Option<String>,
        icon: Option<Rc<RefCell<dyn ControlObject>>>,
        callback: Callback<()>,
        sub_items: Vec<MenuItem>,
    },
    Custom {
        content: Rc<RefCell<dyn ControlObject>>,
        callback: Callback<()>,
        sub_items: Vec<MenuItem>,
    },
}

impl MenuItem {
    pub fn folder(text: &str, sub_items: Vec<MenuItem>) -> Self {
        MenuItem::Text {
            text: text.into(),
            shortcut: None,
            icon: None,
            callback: Callback::empty(),
            sub_items,
        }
    }

    pub fn simple(text: &str, callback: Callback<()>) -> Self {
        MenuItem::Text {
            text: text.into(),
            shortcut: None,
            icon: None,
            callback,
            sub_items: Vec::new(),
        }
    }

    pub fn full(
        text: &str,
        shortcut: Option<String>,
        icon: Option<Rc<RefCell<dyn ControlObject>>>,
        callback: Callback<()>,
    ) -> Self {
        MenuItem::Text {
            text: text.into(),
            shortcut,
            icon,
            callback,
            sub_items: Vec::new(),
        }
    }
}

#[derive(TypedBuilder)]
pub struct Menu {
    #[builder(default = Orientation::Horizontal)]
    pub orientation: Orientation,

    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn to_view(
        self,
        _style: Option<Box<dyn Style<Self>>>,
        context: ViewContext,
    ) -> Rc<RefCell<dyn ControlObject>> {
        // menu is active when tapped
        let is_menu_active_prop = Property::new(false);

        let mut content_prop = ObservableVec::new();
        let mut close_item_popup_callbacks = Vec::new();
        let mut close_siblings_callbacks = Vec::new();

        let menu: Rc<RefCell<dyn ControlObject>> = ui!(
            Border {
                border_type: BorderType::None,
                Style: Default { background_color: [1.0f32, 1.0f32, 1.0f32, 0.8f32], },

                StackPanel {
                    orientation: self.orientation,

                    &content_prop,
                }
            }
        );

        let uncovered_controls: Vec<_> = vec![Rc::downgrade(&menu)];

        for item in self.items.into_iter() {
            let close_siblings_callback_rc = Rc::new(RefCell::new(Callback::empty()));
            let (view, close_item_popup_callback) = item.to_view(
                true,
                &is_menu_active_prop,
                &uncovered_controls,
                &close_siblings_callback_rc,
            );
            content_prop.push(view);
            close_item_popup_callbacks.push(close_item_popup_callback);
            close_siblings_callbacks.push(close_siblings_callback_rc);
        }

        // setup sibling closing logic
        for i in 0..close_siblings_callbacks.len() {
            let mut close_item_popup_callbacks_for_i = Vec::new();
            for j in 0..close_item_popup_callbacks.len() {
                if j != i {
                    close_item_popup_callbacks_for_i.push(close_item_popup_callbacks[j].clone());
                }
            }

            close_siblings_callbacks[i].borrow_mut().set(move |_| {
                for i in 0..close_item_popup_callbacks_for_i.len() {
                    close_item_popup_callbacks_for_i[i].emit(());
                }
            });
        }

        let data_holder = DataHolder { data: () };
        data_holder.to_view(
            None,
            ViewContext {
                attached_values: context.attached_values,
                children: Children::SingleStatic(menu),
            },
        )
    }
}

impl MenuItem {
    pub fn to_view(
        self,
        is_top: bool,
        is_menu_active_prop: &Property<bool>,
        uncovered_controls: &Vec<Weak<RefCell<dyn ControlObject>>>,
        close_siblings_callback_rc: &Rc<RefCell<Callback<()>>>,
    ) -> (Rc<RefCell<dyn ControlObject>>, Callback<()>) {
        match self {
            MenuItem::Separator => {
                let separator: Rc<RefCell<dyn ControlObject>> = ui! {
                    Text {
                        Style: Default { color: [0.0f32, 0.0f32, 0.0f32, 1.0f32] },
                        text: "---------"
                    }
                };
                (separator, Callback::empty())
            }

            MenuItem::Text {
                text,
                shortcut,
                icon,
                callback,
                sub_items,
            } => {
                let has_sub_items = sub_items.len() > 0;

                let mut is_open_prop = Property::new(false);
                let mut background_property = Property::new([0.0f32, 0.0f32, 0.0f32, 0.0f32]);
                let mut foreground_property = Property::new([0.0f32, 0.0f32, 0.0f32, 1.0f32]);

                let mut on_hover_callback = Callback::empty();
                let mut on_tap_down_callback = Callback::empty();

                if is_top {
                    // top bar menu case

                    // open sub menu on tap down
                    let mut is_menu_active_prop_clone = is_menu_active_prop.clone();
                    let mut is_open_prop_clone = is_open_prop.clone();
                    on_tap_down_callback.set(move |_| {
                        if has_sub_items {
                            is_menu_active_prop_clone.set(true);
                            is_open_prop_clone.set(true);
                        } else {
                            // execute menu command
                            callback.emit(());
                        }
                    });

                    // hover highlights items even when menu is not active
                    let mut background_property_clone = background_property.clone();
                    let mut foreground_property_clone = foreground_property.clone();
                    let mut is_menu_active_prop_clone = is_menu_active_prop.clone();
                    let mut is_open_prop_clone = is_open_prop.clone();
                    let close_siblings_callback_clone = close_siblings_callback_rc.clone();
                    on_hover_callback.set(move |value| {
                        background_property_clone.set(
                            if value || is_menu_active_prop_clone.get() {
                                [0.0f32, 0.0f32, 0.0f32, 0.8f32]
                            } else {
                                [0.0f32, 0.0f32, 0.0f32, 0.0f32]
                            },
                        );
                        foreground_property_clone.set(
                            if value || is_menu_active_prop_clone.get() {
                                [1.0f32, 1.0f32, 0.0f32, 1.0f32]
                            } else {
                                [0.0f32, 0.0f32, 0.0f32, 1.0f32]
                            },
                        );

                        if value && is_menu_active_prop_clone.get() {
                            // close all the other popups on the same level (siblings)
                            close_siblings_callback_clone.borrow().emit(());

                            // open popup if there are sub items
                            if has_sub_items {
                                is_open_prop_clone.set(true);
                            }
                        }
                    });
                } else {
                    let mut background_property_clone = background_property.clone();
                    let mut foreground_property_clone = foreground_property.clone();
                    let mut is_open_prop_clone = is_open_prop.clone();
                    let close_siblings_callback_clone = close_siblings_callback_rc.clone();
                    on_hover_callback.set(move |value| {
                        background_property_clone.set(if value || has_sub_items {
                            [0.0f32, 0.0f32, 0.0f32, 0.8f32]
                        } else {
                            [0.0f32, 0.0f32, 0.0f32, 0.0f32]
                        });
                        foreground_property_clone.set(if value || has_sub_items {
                            [1.0f32, 1.0f32, 0.0f32, 1.0f32]
                        } else {
                            [0.0f32, 0.0f32, 0.0f32, 1.0f32]
                        });

                        if value {
                            // close all the other popups on the same level (siblings)
                            close_siblings_callback_clone.borrow().emit(());

                            // open popup if there are sub items
                            if has_sub_items {
                                is_open_prop_clone.set(true);
                            }
                        }
                    });

                    let mut is_menu_active_prop_clone = is_menu_active_prop.clone();
                    on_tap_down_callback.set(move |_| {
                        if !has_sub_items {
                            // close menu
                            is_menu_active_prop_clone.set(false);
                            // execute menu command
                            callback.emit(());
                        }
                    });
                }

                let title_content: Rc<RefCell<dyn ControlObject>> = if is_top {
                    ui!(Text {
                        Row: 0,
                        Column: 1,
                        Margin: Thickness::new(5.0f32, 0.0f32, 5.0f32, 0.0f32),
                        Style: Dynamic {
                            color: foreground_property.clone()
                        },
                        text: text
                    })
                } else {
                    ui!(
                        Grid {
                            columns: 4,
                            widths: vec![
                                (0, Length::Exact(25.0f32)),
                                (1, Length::Fill(1.0f32)),
                                (2, Length::Auto),
                                (3, Length::Exact(25.0f32)),
                            ],

                            Text {
                                Row: 0, Column: 1,
                                HorizontalAlignment: Alignment::Start,
                                Style: Dynamic { color: foreground_property.clone() },

                                text: text
                            },

                            Text {
                                Row: 0, Column: 3,
                                Style: Dynamic { color: foreground_property.clone() },
                                text: if sub_items.len() > 0 { ">" } else { "" },
                            }
                        }
                    )
                };

                // return callback that closes the popup
                let mut close_popup_callback = Callback::empty();

                let popup = if sub_items.len() == 0 {
                    Children::None
                } else {
                    let mut sub_content_prop = ObservableVec::new();

                    let popup_placement = if is_top {
                        PopupPlacement::BelowOrAboveParent
                    } else {
                        PopupPlacement::LeftOrRightParent
                    };

                    let mut background_property_clone = background_property.clone();
                    let mut foreground_property_clone = foreground_property.clone();
                    let mut is_menu_active_prop_clone = is_menu_active_prop.clone();
                    let popup_close_subscription = is_open_prop.on_changed(move |value| {
                        if value == false {
                            background_property_clone.set([0.0f32, 0.0f32, 0.0f32, 0.0f32]);
                            foreground_property_clone.set([0.0f32, 0.0f32, 0.0f32, 1.0f32]);
                        }

                        if is_top {
                            //is_menu_active_prop_clone.set(false);
                        }
                    });

                    let popup_content: Rc<RefCell<dyn ControlObject>> = ui!(
                        Border {
                            Style: Default { background_color: [1.0f32, 1.0f32, 1.0f32, 0.8f32], },

                            Grid {
                                columns: 1,
                                default_width: Length::Fill(1.0f32),
                                default_height: Length::Auto,

                                &sub_content_prop,
                            }
                        }
                    );

                    let mut close_item_popup_callbacks = Vec::new();
                    let mut close_siblings_callbacks = Vec::new();

                    let mut uncovered_controls = uncovered_controls.to_vec();
                    uncovered_controls.push(Rc::downgrade(&popup_content));
                    for item in sub_items.into_iter() {
                        let close_siblings_callback_rc = Rc::new(RefCell::new(Callback::empty()));
                        let (view, close_item_popup_callback) = item.to_view(
                            false,
                            &is_menu_active_prop,
                            &uncovered_controls,
                            &close_siblings_callback_rc,
                        );
                        sub_content_prop.push(view);
                        close_item_popup_callbacks.push(close_item_popup_callback);
                        close_siblings_callbacks.push(close_siblings_callback_rc);
                    }

                    // setup sibling closing logic
                    for i in 0..close_siblings_callbacks.len() {
                        let mut close_item_popup_callbacks_for_i = Vec::new();
                        for j in 0..close_item_popup_callbacks.len() {
                            if j != i {
                                close_item_popup_callbacks_for_i
                                    .push(close_item_popup_callbacks[j].clone());
                            }
                        }

                        close_siblings_callbacks[i].borrow_mut().set(move |_| {
                            for i in 0..close_item_popup_callbacks_for_i.len() {
                                close_item_popup_callbacks_for_i[i].emit(());
                            }
                        });
                    }

                    // when clicked outside last open submenu
                    // make whole menu inactive (and close all submenu windows)
                    let mut auto_hide_occured_callback = Callback::empty();
                    let mut is_menu_active_prop_clone = is_menu_active_prop.clone();
                    auto_hide_occured_callback.set(move |_| {
                        is_menu_active_prop_clone.set(false);
                    });

                    let mut is_open_prop_clone = is_open_prop.clone();
                    let is_menu_active_prop_changed =
                        is_menu_active_prop.on_changed(move |value| {
                            if !value {
                                is_open_prop_clone.set(false);
                            }
                        });

                    // return callback that closes the popup
                    let mut is_open_prop_clone = is_open_prop.clone();
                    close_popup_callback.set(move |_| {
                        // close this popup
                        is_open_prop_clone.set(false);
                        // close all sub-popups
                        for subsibling in &close_siblings_callbacks {
                            subsibling.borrow().emit(());
                        }
                    });

                    let data_holder = DataHolder {
                        data: (popup_close_subscription, is_menu_active_prop_changed),
                    }
                    .to_view(
                        None,
                        ViewContext {
                            attached_values: TypeMap::new(),
                            children: Children::None,
                        },
                    );

                    let popup = ui!(Popup {
                        is_open: is_open_prop,
                        placement: popup_placement,
                        auto_hide: PopupAutoHide::ClickedOutside,
                        auto_hide_occured: auto_hide_occured_callback,
                        uncovered_controls: uncovered_controls,

                        popup_content,

                        data_holder,
                    });

                    Children::SingleStatic(popup)
                };

                let content = ui!(
                    GestureArea {
                        hover_change: on_hover_callback,
                        tap_down: on_tap_down_callback,

                        Border {
                            border_type: BorderType::None,
                            Style: Default { background_color: background_property },

                            title_content,
                        },

                        popup,
                    }
                );

                (content, close_popup_callback)
            }

            MenuItem::Custom {
                content,
                callback,
                sub_items,
            } => (content, Callback::empty()),
        }
    }
}
