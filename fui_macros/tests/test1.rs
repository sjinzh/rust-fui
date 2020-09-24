use fui_macros::ui;
use std::cell::RefCell;
use std::rc::Rc;
use typed_builder::TypedBuilder;
use typemap::{Key, TypeMap};

/// Children collection of a control.
///
/// The collection is an enum to make it optimized for the most common cases.
pub enum Children {
    /// The collection has no items.
    None,

    /// The collection has a single child.
    SingleStatic(Rc<RefCell<dyn ControlObject>>),

    /// The children comes from a single observable collection.
    SingleDynamic(Box<dyn ObservableCollection<Rc<RefCell<dyn ControlObject>>>>),

    /// The collection is a list of controls.
    MultipleStatic(Vec<Rc<RefCell<dyn ControlObject>>>),

    /// The collection is a mix of controls and observable collections.
    MultipleMixed(Vec<SubChildren>),
}

pub enum SubChildren {
    SingleStatic(Rc<RefCell<dyn ControlObject>>),
    SingleDynamic(Box<dyn ObservableCollection<Rc<RefCell<dyn ControlObject>>>>),
    MultipleStatic(Vec<Rc<RefCell<dyn ControlObject>>>),
}

impl Children {
    /// Creates an empty children collection.
    pub fn new() -> Self {
        Children::None
    }

    /// Constructs Children collection from
    /// vector of Children collections.
    pub fn from(children_vec: Vec<Children>) -> Self {
        let mut iter = children_vec.into_iter();
        if let Some(next) = iter.next() {
            let mut result = next;
            while let Some(next) = iter.next() {
                result = result.append(next)
            }
            result
        } else {
            Children::None
        }
    }

    /// Returns number of controls in the children collection.
    pub fn len(&self) -> usize {
        match self {
            Children::None => 0,
            Children::SingleStatic(_) => 1,
            Children::SingleDynamic(x) => x.len(),
            Children::MultipleStatic(x) => x.len(),
            Children::MultipleMixed(x) => x.iter().map(|i| i.len()).sum(),
        }
    }

    /// Tries to get Rc reference to the control at the `index` position.
    pub fn get(&self, mut index: usize) -> Option<Rc<RefCell<dyn ControlObject>>> {
        match self {
            Children::None => None,
            Children::SingleStatic(x) => {
                if index == 0 {
                    Some(x.clone())
                } else {
                    None
                }
            }
            Children::SingleDynamic(x) => x.get(index),
            Children::MultipleStatic(x) => x.get(index),
            Children::MultipleMixed(x) => {
                for sub_children in x {
                    let len = sub_children.len();
                    if index < len {
                        return sub_children.get(index);
                    } else {
                        index -= len;
                    }
                }
                None
            }
        }
    }

    /// Appends another children collection to self.
    /// Returns new instance of an enum.  
    fn append(self, children: Children) -> Self {
        match children {
            Children::None => self,
            Children::SingleStatic(x) => self.add(Children::SingleStatic(x)),
            Children::SingleDynamic(x) => self.add(Children::SingleDynamic(x)),
            Children::MultipleStatic(x) => {
                let mut result = self;
                for el in x.into_iter() {
                    result = result.add(Children::SingleStatic(el));
                }
                result
            }
            Children::MultipleMixed(x) => {
                let mut result = self;
                for el in x.into_iter() {
                    match el {
                        SubChildren::SingleStatic(x) => {
                            result = result.add(Children::SingleStatic(x))
                        }
                        SubChildren::SingleDynamic(x) => {
                            result = result.add(Children::SingleDynamic(x))
                        }
                        SubChildren::MultipleStatic(x) => {
                            for el in x.into_iter() {
                                result = result.add(Children::SingleStatic(el));
                            }
                        }
                    }
                }
                result
            }
        }
    }

    /// Adds a control entry (single control or single observable collection)
    /// to the children collection.
    /// Returns new instance of an enum.
    fn add(self, child: Children) -> Self {
        match self {
            Children::None => match child {
                Children::SingleStatic(c) => Children::SingleStatic(c),

                Children::SingleDynamic(c) => Children::SingleDynamic(c),

                _ => unreachable!(),
            },

            Children::SingleStatic(x) => match child {
                Children::SingleStatic(c) => Children::MultipleStatic(vec![x, c]),

                Children::SingleDynamic(c) => Children::MultipleMixed(vec![
                    SubChildren::SingleStatic(x),
                    SubChildren::SingleDynamic(c),
                ]),

                _ => unreachable!(),
            },

            Children::SingleDynamic(x) => match child {
                Children::SingleStatic(c) => Children::MultipleMixed(vec![
                    SubChildren::SingleDynamic(x),
                    SubChildren::SingleStatic(c),
                ]),

                Children::SingleDynamic(c) => Children::MultipleMixed(vec![
                    SubChildren::SingleDynamic(x),
                    SubChildren::SingleDynamic(c),
                ]),

                _ => unreachable!(),
            },

            Children::MultipleStatic(mut x) => match child {
                Children::SingleStatic(c) => {
                    x.push(c);
                    Children::MultipleStatic(x)
                }

                Children::SingleDynamic(c) => Children::MultipleMixed(vec![
                    SubChildren::MultipleStatic(x),
                    SubChildren::SingleDynamic(c),
                ]),

                _ => unreachable!(),
            },

            Children::MultipleMixed(mut x) => match child {
                Children::SingleStatic(c) => {
                    if let Some(last) = x.pop() {
                        match last {
                            SubChildren::SingleStatic(l) => {
                                x.push(SubChildren::MultipleStatic(vec![l, c]));
                                Children::MultipleMixed(x)
                            }

                            SubChildren::SingleDynamic(l) => {
                                x.push(SubChildren::SingleDynamic(l));
                                x.push(SubChildren::SingleStatic(c));
                                Children::MultipleMixed(x)
                            }

                            SubChildren::MultipleStatic(mut l) => {
                                l.push(c);
                                x.push(SubChildren::MultipleStatic(l));
                                Children::MultipleMixed(x)
                            }
                        }
                    } else {
                        Children::SingleStatic(c)
                    }
                }

                Children::SingleDynamic(c) => {
                    if let Some(last) = x.pop() {
                        match last {
                            SubChildren::SingleStatic(l) => {
                                x.push(SubChildren::SingleStatic(l));
                                x.push(SubChildren::SingleDynamic(c));
                                Children::MultipleMixed(x)
                            }

                            SubChildren::SingleDynamic(l) => {
                                x.push(SubChildren::SingleDynamic(l));
                                x.push(SubChildren::SingleDynamic(c));
                                Children::MultipleMixed(x)
                            }

                            SubChildren::MultipleStatic(l) => {
                                x.push(SubChildren::MultipleStatic(l));
                                x.push(SubChildren::SingleDynamic(c));
                                Children::MultipleMixed(x)
                            }
                        }
                    } else {
                        Children::SingleDynamic(c)
                    }
                }

                _ => unreachable!(),
            },
        }
    }
}

/// Converts a single control to Children collection.
impl From<Rc<RefCell<dyn ControlObject>>> for Children {
    fn from(item: Rc<RefCell<dyn ControlObject>>) -> Children {
        Children::SingleStatic(item)
    }
}

/// Converts a single control to ChildEntry.
impl<T: 'static + ControlObject> From<Rc<RefCell<T>>> for Children {
    fn from(item: Rc<RefCell<T>>) -> Children {
        Children::SingleStatic(item)
    }
}

/// Converts an observable collection to ChildEntry.
impl<T: Into<Box<dyn ObservableCollection<Rc<RefCell<dyn ControlObject>>>>>> From<T> for Children {
    fn from(item: T) -> Children {
        Children::SingleDynamic(item.into())
    }
}

pub struct ChildrenIterator<'a> {
    source: &'a Children,
    pos: usize,
    len: usize,
}

impl<'a> Iterator for ChildrenIterator<'a> {
    type Item = Rc<RefCell<dyn ControlObject>>;

    fn next(&mut self) -> Option<Rc<RefCell<dyn ControlObject>>> {
        if self.pos < self.len {
            self.pos += 1;
            self.source.get(self.pos - 1)
        } else {
            None
        }
    }
}

impl<'a> DoubleEndedIterator for ChildrenIterator<'a> {
    fn next_back(&mut self) -> Option<Rc<RefCell<dyn ControlObject>>> {
        if self.len > self.pos {
            self.len -= 1;
            self.source.get(self.len)
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = Rc<RefCell<dyn ControlObject>>;
    type IntoIter = ChildrenIterator<'a>;

    fn into_iter(self) -> ChildrenIterator<'a> {
        ChildrenIterator {
            source: self,
            pos: 0,
            len: self.len(),
        }
    }
}

impl SubChildren {
    pub fn len(&self) -> usize {
        match self {
            SubChildren::SingleStatic(_) => 1,
            SubChildren::SingleDynamic(x) => x.len(),
            SubChildren::MultipleStatic(x) => x.len(),
        }
    }

    pub fn get(&self, index: usize) -> Option<Rc<RefCell<dyn ControlObject>>> {
        match self {
            SubChildren::SingleStatic(x) => {
                if index == 0 {
                    Some(x.clone())
                } else {
                    None
                }
            }
            SubChildren::SingleDynamic(x) => x.get(index),
            SubChildren::MultipleStatic(x) => x.get(index),
        }
    }
}

pub trait ObservableCollection<T: 'static + Clone> {
    fn iter1<'a>(&'a self) -> ::std::slice::Iter<'a, T>;

    fn len(&self) -> usize;

    fn get(&self, index: usize) -> Option<T>;
}

///
/// ObservableCollection for Vec.
///
impl ObservableCollection<Rc<RefCell<dyn ControlObject>>> for Vec<Rc<RefCell<dyn ControlObject>>> {
    fn iter1<'a>(&'a self) -> ::std::slice::Iter<'a, Rc<RefCell<dyn ControlObject>>> {
        self.iter()
    }

    fn len(&self) -> usize {
        Vec::<Rc<RefCell<dyn ControlObject>>>::len(&self)
    }

    fn get(&self, index: usize) -> Option<Rc<RefCell<dyn ControlObject>>> {
        self.as_slice().get(index).map(|el| el.clone())
    }
}

// attached value Row of type i32
struct Row;
impl Key for Row {
    type Value = i32;
}

pub trait ControlObject {
    fn draw(&mut self) -> String;
}

pub struct ViewContext {
    attached_values: TypeMap,
    children: Children,
}

pub trait Style<D> {
    fn draw(&self, data: &mut D) -> String;
}

pub struct StyledControl<D> {
    pub data: D,
    pub style: Box<dyn Style<D>>,
    pub attached_values: TypeMap,
    pub children: Children,
}

impl<D: 'static> StyledControl<D> {
    pub fn new(
        data: D,
        style: Box<dyn Style<D>>,
        attached_values: TypeMap,
        children: Children,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(StyledControl {
            data: data,
            attached_values: attached_values,
            style,
            children: children,
        }))
    }
}

impl<D: 'static> ControlObject for StyledControl<D> {
    fn draw(&mut self) -> String {
        let name = self.style.draw(&mut self.data);
        let mut attached_values = "".to_string();
        if let Some(row_attached_value) = self.attached_values.get::<Row>() {
            attached_values += &format!(".Row({})", row_attached_value);
        }

        let children = {
            let vec: Vec<String> = (&self.children)
                .into_iter()
                .map(|c| c.borrow_mut().draw())
                .collect();
            vec.join(",")
        };

        name + &attached_values + "{" + &children + "}"
    }
}

#[derive(Debug, TypedBuilder)]
pub struct Horizontal {
    #[builder(default = 0)]
    pub spacing: i32,
}

impl Horizontal {
    pub fn to_view(
        self,
        style: Option<Box<dyn Style<Self>>>,
        context: ViewContext,
    ) -> Rc<RefCell<StyledControl<Self>>> {
        StyledControl::new(
            self,
            style.unwrap_or_else(|| {
                Box::new(DefaultHorizontalStyle::new(
                    DefaultHorizontalStyleParams::builder().build(),
                ))
            }),
            context.attached_values,
            context.children,
        )
    }
}

#[derive(TypedBuilder)]
pub struct DefaultHorizontalStyleParams {}

pub struct DefaultHorizontalStyle {}

impl DefaultHorizontalStyle {
    pub fn new(_params: DefaultHorizontalStyleParams) -> Self {
        DefaultHorizontalStyle {}
    }
}

impl Style<Horizontal> for DefaultHorizontalStyle {
    fn draw(&self, data: &mut Horizontal) -> String {
        format!("Horizontal({})", data.spacing)
    }
}

#[derive(Debug, TypedBuilder)]
pub struct Button {}

impl Button {
    pub fn to_view(
        self,
        style: Option<Box<dyn Style<Self>>>,
        context: ViewContext,
    ) -> Rc<RefCell<StyledControl<Self>>> {
        StyledControl::new(
            self,
            style.unwrap_or_else(|| {
                Box::new(DefaultButtonStyle::new(
                    DefaultButtonStyleParams::builder().build(),
                ))
            }),
            context.attached_values,
            context.children,
        )
    }
}

#[derive(TypedBuilder)]
pub struct DefaultButtonStyleParams {}

pub struct DefaultButtonStyle {}

impl DefaultButtonStyle {
    pub fn new(_params: DefaultButtonStyleParams) -> Self {
        DefaultButtonStyle {}
    }
}

impl Style<Button> for DefaultButtonStyle {
    fn draw(&self, _data: &mut Button) -> String {
        "Button".to_string()
    }
}

#[derive(Debug, TypedBuilder)]
pub struct Text {
    pub text: String,
}

impl Text {
    fn to_view(
        self,
        style: Option<Box<dyn Style<Self>>>,
        context: ViewContext,
    ) -> Rc<RefCell<StyledControl<Self>>> {
        StyledControl::new(
            self,
            style.unwrap_or_else(|| {
                Box::new(DefaultTextStyle::new(
                    DefaultTextStyleParams::builder().build(),
                ))
            }),
            context.attached_values,
            context.children,
        )
    }
}

#[derive(TypedBuilder)]
pub struct DefaultTextStyleParams {}

pub struct DefaultTextStyle {}

impl DefaultTextStyle {
    pub fn new(_params: DefaultTextStyleParams) -> Self {
        DefaultTextStyle {}
    }
}

impl Style<Text> for DefaultTextStyle {
    fn draw(&self, data: &mut Text) -> String {
        format!("Text(\"{}\")", data.text)
    }
}

#[test]
fn test1() {
    let control = ui!(
        Horizontal {
            Row: 1,
            spacing: 4,
            Button { Text { text: "Button".to_string() } },
            Text { text: "Label".to_string() }
        }
    );

    let mut control: std::cell::RefMut<dyn ControlObject> = control.borrow_mut();
    assert_eq!(
        "Horizontal(4).Row(1){Button{Text(\"Button\"){}},Text(\"Label\"){}}",
        control.draw()
    );

    //println!("{}", control.draw());
    //println!("{:?}", control);
}
