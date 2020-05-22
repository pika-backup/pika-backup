#!/usr/bin/python3

import xml.etree.ElementTree as ET

UI_FILES = [
    ["About", "ui/about.ui"],
    ["Main", "ui/main.ui"],
    ["NewBackup", "ui/new.ui"],
    ["Storage", "ui/storage.ui"],
    ["EncryptionPassword", "ui/password.ui"],
]

SRC_PATH = "src/ui/builder.rs"

class Item:
    def __init__(self, id, type):
        self.id = id
        self.type = type

def objects(path):
    objects = []
    for item in ET.parse("data/" + path).iter():
        if item.tag == "object" and item.get("id"):
            objects.append(Item(item.get("id"), item.get("class")[3:]))
    objects.sort(key=lambda item: item.id)

    return objects

def fn_code(objects):
    template = """
    pub fn {id}(&self) -> gtk::{type} {{
        self.get("{id}")
    }}"""

    code = ""
    for o in objects:
        if code:
            code += "\n"
        code += template.format(**o.__dict__)

    return code

def struct_code(name, path):
    template = \
"""pub struct {name} {{
    builder: gtk::Builder,
}}

impl {name} {{
    pub fn new() -> Self {{
        Self {{
            builder: gtk::Builder::new_from_string(include_str!(concat!(
                data_dir!(),
                "/{path}"
            ))),
        }}
    }}

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {{
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id)
            .unwrap_or_else(|| panic!("Object with id '{{}}' not found in '{path}'", id))
    }}
{fn_code}
}}"""

    code = fn_code(objects(path))
    return template.format(name=name, path=path, fn_code=code)

with open(SRC_PATH, "w") as src:
    first = True
    for x in UI_FILES:
        if first:
            first = False
        else:
            src.write("\n\n")

        src.write(struct_code(*x))

    src.write("\n")
