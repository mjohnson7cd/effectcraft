//! File ▸ Export ▸ Lottie JSON… / File ▸ Import ▸ Lottie… through the command registry.

use serde_json::{Value, json};

use crate::Session;

fn tmp(name: &str) -> String {
    let d = std::env::temp_dir().join(format!("effectcraft-lottie-tests-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join(name).to_string_lossy().to_string()
}

fn session() -> Session {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Main", "width": 320, "height": 180, "frameRate": 30, "duration": 2})).unwrap();
    s.execute("layer.newSolid", json!({"color": "#ff8000", "width": 100, "height": 60})).unwrap();
    s.execute("layer.newShape", json!({})).ok();
    s
}

#[test]
fn export_and_import_lottie_commands() {
    let mut s = session();
    let n_layers = s.active_comp().unwrap().layers.len();
    s.execute("prop.addKey", json!({"layer": "#1", "path": "transform/opacity", "time": 0.0, "value": 0.0})).unwrap();
    s.execute("prop.addKey", json!({"layer": "#1", "path": "transform/opacity", "time": 1.0, "value": 100.0})).unwrap();
    let path = tmp("main.json");
    let r = s.execute_checked("file.exportLottie", json!({"path": path, "includeExpressions": true})).unwrap();
    assert!(r["bytes"].as_u64().unwrap() > 100);
    let doc: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(doc["w"], json!(320));
    assert_eq!(doc["layers"].as_array().unwrap().len(), n_layers);
    // Import as a new comp (undoable).
    let before = s.project.items.len();
    let r = s.execute_checked("file.importLottie", json!({"path": path})).unwrap();
    let cid = effectcraft_project::ItemId(r["comp"].as_u64().unwrap());
    assert_eq!(s.state.active_comp, Some(cid));
    let c = s.project.comp(cid).unwrap();
    assert_eq!(c.layers.len(), n_layers);
    let op = c.layers[0].props.prop("transform/opacity").unwrap();
    assert_eq!(op.keys.len(), 2);
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(s.project.items.len(), before);
    // dotLottie.
    let dot = tmp("main.lottie");
    s.execute("file.exportLottie", json!({"path": dot})).unwrap();
    assert!(std::fs::read(&dot).unwrap().starts_with(b"PK"));
    let r = s.execute("file.importLottie", json!({"path": dot})).unwrap();
    assert!(r["comp"].as_u64().is_some());
    // Unknown parameters are rejected for agents.
    assert!(s.execute_checked("file.exportLottie", json!({"path": path, "bogus": 1})).is_err());
}

#[test]
fn lottie_menu_entries_exist() {
    let entries = crate::menus::entries();
    let find = |cmd: &str| entries.iter().find(|(_, e)| e.command == cmd).map(|(p, e)| (p.join(" > "), e.label.clone()));
    assert_eq!(find("file.exportLottie"), Some(("File > Export".into(), "Lottie JSON...".into())));
    assert_eq!(find("file.importLottie"), Some(("File > Import".into(), "Lottie...".into())));
}

#[test]
fn offset_paths_miter_limit_is_animatable() {
    let mut s = session();
    s.execute("layer.newShape", json!({"kind": "rect", "size": [100, 100]})).unwrap();
    let off = s.execute("layer.addShapeItem", json!({"layer": "#1", "kind": "offset", "group": "contents/group"})).unwrap();
    let path = off["path"].as_str().unwrap();
    s.execute("prop.set", json!({"layer": "#1", "path": format!("{}/amount", path), "value": -4})).unwrap();
    s.execute("prop.set", json!({"layer": "#1", "path": format!("{}/miter", path), "value": 8})).unwrap();
    let export_path = tmp("offset.json");
    s.execute("file.exportLottie", json!({"path": &export_path})).unwrap();
    let doc: Value = serde_json::from_slice(&std::fs::read(&export_path).unwrap()).unwrap();
    let shapes = &doc["layers"][0]["shapes"];
    // Find Offset Paths inside the shape group's items
    let op = shapes.as_array().unwrap().iter()
        .flat_map(|g| g["it"].as_array())
        .flatten()
        .find(|s| s["ty"] == "op")
        .expect("Offset Paths shape not found");
    // ml must be {"a": 0, "k": 8}, not just 8
    assert!(op["ml"].is_object(), "ml is not an animatable value: {:?}", op["ml"]);
    assert_eq!(op["ml"]["a"], json!(0));
    assert_eq!(op["ml"]["k"], json!(8.0));
}
