use std::{collections::HashMap, error::Error, path::PathBuf, sync::atomic::AtomicUsize};

use clap::Parser;
use egui::Widget;
use pkmc_util::nbt::{NBTList, NBT};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(index = 1, num_args=0.., value_delimiter=' ')]
    files: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut nbts = Vec::new();

    args.files.iter().try_for_each(|file| {
        let nbt = NBT::read(std::fs::File::open(file)?)?;
        nbts.push(EditableNBTWrapper::from_nbt(Some(nbt.0), nbt.1));
        Ok::<_, Box<dyn Error>>(())
    })?;

    if nbts.is_empty() {
        nbts.push(EditableNBTWrapper::from_nbt(
            Some("root".to_owned()),
            NBT::Compound(HashMap::from([(
                "message".to_owned(),
                NBT::String("Hello, World!".to_owned()),
            )])),
        ));
    }

    App {
        editor: nbts.into_iter().next().expect("No NBT specified."),
    }
    .run()?;

    Ok(())
}

#[derive(Debug)]
enum EditableNBT {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    // MUST HAVE SAME TYPE
    List(Vec<EditableNBTWrapper>),
    // MUST HAVE UNIQUE KEYS
    Compound(Vec<EditableNBTWrapper>),
    ByteArray(Vec<i8>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl From<NBT> for EditableNBT {
    fn from(value: NBT) -> Self {
        match value {
            NBT::Byte(byte) => EditableNBT::Byte(byte),
            NBT::Short(short) => EditableNBT::Short(short),
            NBT::Int(int) => EditableNBT::Int(int),
            NBT::Long(long) => EditableNBT::Long(long),
            NBT::Float(float) => EditableNBT::Float(float),
            NBT::Double(double) => EditableNBT::Double(double),
            NBT::String(string) => EditableNBT::String(string),
            NBT::List(list) => EditableNBT::List(
                list.into_iter()
                    .map(|v| EditableNBTWrapper::from_nbt(None, v))
                    .collect(),
            ),
            NBT::Compound(compound) => EditableNBT::Compound(
                compound
                    .into_iter()
                    .map(|(k, v)| EditableNBTWrapper::from_nbt(Some(k), v))
                    .collect(),
            ),
            NBT::ByteArray(byte_array) => EditableNBT::ByteArray(byte_array.to_vec()),
            NBT::IntArray(int_array) => EditableNBT::IntArray(int_array.to_vec()),
            NBT::LongArray(long_array) => EditableNBT::LongArray(long_array.to_vec()),
        }
    }
}

impl From<&EditableNBT> for NBT {
    fn from(val: &EditableNBT) -> Self {
        match val {
            EditableNBT::Byte(byte) => NBT::Byte(*byte),
            EditableNBT::Short(short) => NBT::Short(*short),
            EditableNBT::Int(int) => NBT::Int(*int),
            EditableNBT::Long(long) => NBT::Long(*long),
            EditableNBT::Float(float) => NBT::Float(*float),
            EditableNBT::Double(double) => NBT::Double(*double),
            EditableNBT::String(string) => NBT::String(string.to_owned()),
            EditableNBT::List(list) => NBT::List(
                NBTList::try_from(list.iter().map(|v| v.to_nbt().1).collect::<Vec<_>>()).unwrap(),
            ),
            EditableNBT::Compound(compound) => NBT::Compound(
                compound
                    .iter()
                    .map(|v| {
                        let (k, v) = v.to_nbt();
                        (k.unwrap(), v)
                    })
                    .collect(),
            ),
            EditableNBT::ByteArray(byte_array) => {
                NBT::ByteArray(byte_array.clone().into_boxed_slice())
            }
            EditableNBT::IntArray(int_array) => NBT::IntArray(int_array.clone().into_boxed_slice()),
            EditableNBT::LongArray(long_array) => {
                NBT::LongArray(long_array.clone().into_boxed_slice())
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
enum EditableNBTType {
    #[default]
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    String,
    List,
    Compound,
    ByteArray,
    IntArray,
    LongArray,
}

impl EditableNBTType {
    fn to_default_nbt(self) -> NBT {
        match self {
            EditableNBTType::Byte => NBT::Byte(0),
            EditableNBTType::Short => NBT::Short(0),
            EditableNBTType::Int => NBT::Int(0),
            EditableNBTType::Long => NBT::Long(0),
            EditableNBTType::Float => NBT::Float(0.0),
            EditableNBTType::Double => NBT::Double(0.0),
            EditableNBTType::String => NBT::String(String::new()),
            EditableNBTType::List => NBT::List(NBTList::new()),
            EditableNBTType::Compound => NBT::Compound(HashMap::new()),
            EditableNBTType::ByteArray => NBT::ByteArray(vec![0i8; 1].into_boxed_slice()),
            EditableNBTType::IntArray => NBT::IntArray(vec![0i32; 1].into_boxed_slice()),
            EditableNBTType::LongArray => NBT::LongArray(vec![0i64; 1].into_boxed_slice()),
        }
    }

    fn from_nbt(nbt: &EditableNBT) -> Self {
        match nbt {
            EditableNBT::Byte(_) => Self::Byte,
            EditableNBT::Short(_) => Self::Short,
            EditableNBT::Int(_) => Self::Int,
            EditableNBT::Long(_) => Self::Long,
            EditableNBT::Float(_) => Self::Float,
            EditableNBT::Double(_) => Self::Float,
            EditableNBT::String(_) => Self::String,
            EditableNBT::List(_) => Self::List,
            EditableNBT::Compound(_) => Self::Compound,
            EditableNBT::ByteArray(_) => Self::ByteArray,
            EditableNBT::IntArray(_) => Self::IntArray,
            EditableNBT::LongArray(_) => Self::LongArray,
        }
    }
}

#[derive(Debug)]
struct CreateSettings {
    name: String,
    index: usize,
    r#type: EditableNBTType,
}

impl Default for CreateSettings {
    fn default() -> Self {
        Self {
            name: "name".to_string(),
            index: 0,
            r#type: EditableNBTType::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum EditableNBTEvent {
    #[default]
    None,
    Delete,
    SortKeys,
}

#[derive(Debug)]
struct EditableNBTWrapper {
    name: Option<String>,
    value: EditableNBT,
    id: usize,
    event: EditableNBTEvent,
    new: Option<CreateSettings>,
    len_placeholder: usize,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
const ID_EDITABLE_NAME: usize = 0;
const ID_EDITABLE_STRING: usize = 1;

const NUMBER_ARRAY_EDIT_MAX_SIZE: usize = 10_000;

impl EditableNBTWrapper {
    pub fn to_nbt(&self) -> (Option<String>, NBT) {
        (self.name.clone(), (&self.value).into())
    }

    pub fn from_nbt(name: Option<String>, value: NBT) -> Self {
        Self {
            name,
            value: {
                let mut value = EditableNBT::from(value);
                if let EditableNBT::Compound(ref mut compound) = value {
                    compound.sort_by(|a, b| a.name.cmp(&b.name));
                }
                value
            },
            id: ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            event: EditableNBTEvent::None,
            new: None,
            len_placeholder: 0,
        }
    }

    pub fn update(&mut self, ui: &mut egui::Ui) {
        // Erm. . . what the skibidi???????!??!?!? This is so bad i know.
        if let Some(new) = self.new.as_mut() {
            let mut open1 = true;
            let mut open2 = true;

            egui::Window::new("New Entry")
                .resizable([false, false])
                .collapsible(false)
                .open(&mut open1)
                .show(ui.ctx(), |ui| {
                    if matches!(self.value, EditableNBT::Compound(_)) {
                        egui::TextEdit::singleline(&mut new.name).ui(ui);
                    }

                    if let EditableNBT::List(list) = &self.value {
                        if !list.is_empty() {
                            egui::DragValue::new(&mut new.index)
                                .range(0..=list.len())
                                .ui(ui);
                        }
                    }

                    if matches!(self.value, EditableNBT::Compound(_))
                        || matches!(&self.value, EditableNBT::List(list) if list.is_empty())
                    {
                        egui::ComboBox::from_label("Type")
                            .selected_text(format!("{:?}", new.r#type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut new.r#type, EditableNBTType::Byte, "Byte");
                                ui.selectable_value(
                                    &mut new.r#type,
                                    EditableNBTType::Short,
                                    "Short",
                                );
                                ui.selectable_value(&mut new.r#type, EditableNBTType::Int, "Int");
                                ui.selectable_value(&mut new.r#type, EditableNBTType::Long, "Long");
                                ui.selectable_value(
                                    &mut new.r#type,
                                    EditableNBTType::Float,
                                    "Float",
                                );
                                ui.selectable_value(
                                    &mut new.r#type,
                                    EditableNBTType::Double,
                                    "Double",
                                );
                                ui.selectable_value(
                                    &mut new.r#type,
                                    EditableNBTType::String,
                                    "String",
                                );
                                ui.selectable_value(&mut new.r#type, EditableNBTType::List, "List");
                                ui.selectable_value(
                                    &mut new.r#type,
                                    EditableNBTType::Compound,
                                    "Compound",
                                );
                                ui.selectable_value(
                                    &mut new.r#type,
                                    EditableNBTType::ByteArray,
                                    "ByteArray",
                                );
                                ui.selectable_value(
                                    &mut new.r#type,
                                    EditableNBTType::IntArray,
                                    "IntArray",
                                );
                                ui.selectable_value(
                                    &mut new.r#type,
                                    EditableNBTType::LongArray,
                                    "LongArray",
                                );
                            });
                    }

                    match &mut self.value {
                        EditableNBT::Compound(compound) => {
                            if ui.button("Create").clicked() {
                                let mut value = EditableNBTWrapper::from_nbt(
                                    Some(new.name.clone()),
                                    new.r#type.to_default_nbt(),
                                );
                                value.event = EditableNBTEvent::SortKeys;
                                compound.push(value);

                                open2 = false;
                            }
                        }
                        EditableNBT::List(list) => {
                            if ui.button("Create").clicked() {
                                list.insert(
                                    new.index,
                                    EditableNBTWrapper::from_nbt(
                                        None,
                                        if let Some(first) = list.first() {
                                            EditableNBTType::from_nbt(&first.value)
                                        } else {
                                            new.r#type
                                        }
                                        .to_default_nbt(),
                                    ),
                                );
                            }
                        }
                        _ => unreachable!(),
                    }
                });

            if !(open1 && open2) {
                self.new = None;
            }
        }

        ui.with_layout(
            if matches!(
                self.value,
                EditableNBT::Compound(_)
                    | EditableNBT::List(_)
                    | EditableNBT::ByteArray(_)
                    | EditableNBT::IntArray(_)
                    | EditableNBT::LongArray(_)
            ) {
                egui::Layout::top_down(egui::Align::Min)
            } else {
                egui::Layout::left_to_right(egui::Align::Min)
            },
            |ui| {
                if let Some(name) = self.name.as_mut() {
                    // FIXME: WHY DOES THIS STILL LOSE THE CONTEXT MENU WHEN RENAMING??
                    // (e.g.: Change from name 'a' to 'z' for keys to be sorted)
                    ui.push_id(egui::Id::new([self.id, ID_EDITABLE_NAME]), |ui| {
                        egui::Label::new(name.clone()).ui(ui).context_menu(|ui| {
                            if egui::TextEdit::singleline(name).ui(ui).changed() {
                                self.event = EditableNBTEvent::SortKeys;
                            }
                        });
                    });
                }

                match &mut self.value {
                    EditableNBT::Byte(byte) => {
                        egui::DragValue::new(byte).ui(ui);
                    }
                    EditableNBT::Short(short) => {
                        egui::DragValue::new(short).ui(ui);
                    }
                    EditableNBT::Int(int) => {
                        egui::DragValue::new(int).ui(ui);
                    }
                    EditableNBT::Long(long) => {
                        // FIXME: The underlying egui type for the drag value seems to be f64.
                        // So converting to f64, then back to i64, causes some precision loss.
                        egui::DragValue::new(long).ui(ui);
                    }
                    EditableNBT::Float(float) => {
                        egui::DragValue::new(float).speed(0.1).ui(ui);
                    }
                    EditableNBT::Double(double) => {
                        egui::DragValue::new(double).speed(0.1).ui(ui);
                    }
                    EditableNBT::String(string) => {
                        let id = egui::Id::new([self.id, ID_EDITABLE_STRING]);
                        if string.contains('\n')
                            // Allow changing to multiline
                            || ui.input(|i| i.key_down(egui::Key::Enter))
                            && ui.memory(|m| m.has_focus(id))
                        {
                            egui::TextEdit::multiline(string).id(id)
                        } else {
                            egui::TextEdit::singleline(string).id(id)
                        }
                        .ui(ui);
                    }
                    EditableNBT::List(list) => {
                        ui.horizontal(|ui| {
                            ui.label("[");
                            if ui.button("+").clicked() {
                                self.new = Some(Default::default());
                            }
                        });
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.vertical(|ui| {
                                    list.retain(|v| v.event != EditableNBTEvent::Delete);
                                    let mut i = 0;
                                    while i < list.len() {
                                        ui.horizontal(|ui| {
                                            if ui.button("-").clicked() {
                                                list.remove(i);
                                            } else {
                                                list[i].update(ui);
                                                i += 1;
                                            }
                                        });
                                    }
                                });
                            });
                            ui.label("]");
                        });
                    }
                    EditableNBT::Compound(compound) => {
                        ui.horizontal(|ui| {
                            ui.label("{");
                            if ui.button("+").clicked() {
                                self.new = Some(Default::default());
                            }
                        });
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.vertical(|ui| {
                                    compound.retain(|v| v.event != EditableNBTEvent::Delete);
                                    if compound
                                        .iter()
                                        .any(|v| v.event == EditableNBTEvent::SortKeys)
                                    {
                                        compound.sort_by(|a, b| a.name.cmp(&b.name));
                                        compound.iter_mut().for_each(|v| {
                                            if v.event == EditableNBTEvent::SortKeys {
                                                v.event = EditableNBTEvent::None;
                                            }
                                        });
                                    }
                                    let mut i = 0;
                                    while i < compound.len() {
                                        ui.horizontal(|ui| {
                                            if ui.button("-").clicked() {
                                                compound.remove(i);
                                            } else {
                                                compound[i].update(ui);
                                                i += 1;
                                            }
                                        });
                                    }
                                });
                            });
                            ui.label("}");
                        });
                    }
                    EditableNBT::ByteArray(byte_array) => {
                        ui.horizontal(|ui| {
                            ui.label("[");
                            let response = egui::DragValue::new(&mut self.len_placeholder)
                                .range(0..=NUMBER_ARRAY_EDIT_MAX_SIZE)
                                .ui(ui);
                            if response.lost_focus() || response.drag_stopped() {
                                byte_array.resize(self.len_placeholder, 0);
                            }
                            if !response.has_focus() && !response.dragged() {
                                self.len_placeholder = byte_array.len();
                            }
                        });
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.horizontal_wrapped(|ui| {
                                    for byte in byte_array {
                                        egui::DragValue::new(byte).ui(ui);
                                    }
                                });
                            });
                            ui.label("]b");
                        });
                    }
                    EditableNBT::IntArray(int_array) => {
                        ui.horizontal(|ui| {
                            ui.label("[");
                            let response = egui::DragValue::new(&mut self.len_placeholder)
                                .range(0..=NUMBER_ARRAY_EDIT_MAX_SIZE)
                                .ui(ui);
                            if response.lost_focus() || response.drag_stopped() {
                                int_array.resize(self.len_placeholder, 0);
                            }
                            if !response.has_focus() && !response.dragged() {
                                self.len_placeholder = int_array.len();
                            }
                        });
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.horizontal_wrapped(|ui| {
                                    for int in int_array {
                                        egui::DragValue::new(int).ui(ui);
                                    }
                                });
                            });
                            ui.label("]i");
                        });
                    }
                    EditableNBT::LongArray(long_array) => {
                        ui.horizontal(|ui| {
                            ui.label("[");
                            let response = egui::DragValue::new(&mut self.len_placeholder)
                                .range(0..=NUMBER_ARRAY_EDIT_MAX_SIZE)
                                .ui(ui);
                            if response.lost_focus() || response.drag_stopped() {
                                long_array.resize(self.len_placeholder, 0);
                            }
                            if !response.has_focus() && !response.dragged() {
                                self.len_placeholder = long_array.len();
                            }
                        });
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.horizontal_wrapped(|ui| {
                                    for long in long_array {
                                        // FIXME: The underlying egui type for the drag value seems to be f64.
                                        // So converting to f64, then back to i64, causes some precision loss.
                                        egui::DragValue::new(long).ui(ui);
                                    }
                                });
                            });
                            ui.label("]l");
                        });
                    }
                };
            },
        );
    }
}

#[derive(Debug)]
struct App {
    editor: EditableNBTWrapper,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            if ui.button("Save As").clicked() {
                if let Some(path) = rfd::FileDialog::new().set_file_name("data.nbt").save_file() {
                    let nbt = self.editor.to_nbt();
                    let mut encoded = Vec::new();
                    NBT::write(&nbt.1, &nbt.0.unwrap(), &mut encoded).unwrap();
                    std::fs::write(path, encoded).unwrap();
                }
            }
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()))
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.editor.update(ui);
                    });
                });
            });
    }
}

impl App {
    pub fn run(self) -> Result<(), Box<dyn Error>> {
        eframe::run_native(
            "nbt-editor",
            eframe::NativeOptions {
                run_and_return: true,
                viewport: egui::ViewportBuilder::default()
                    .with_title("nbt-editor")
                    .with_resizable(true)
                    .with_min_inner_size(egui::vec2(128.0, 128.0)),
                ..Default::default()
            },
            Box::new(|_| Ok(Box::new(self))),
        )?;
        Ok(())
    }
}
