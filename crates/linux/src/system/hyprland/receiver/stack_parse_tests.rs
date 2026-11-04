use super::*;

const LAYERS: &[u8] = br#"{
 "Virtual-1":{"levels":{
  "0":[{"address":"0x1","x":0,"y":0,"w":1280,"h":800,"namespace":"omarchy-background","pid":724}],
  "1":[],
  "2":[{"address":"0x2","x":0,"y":0,"w":1280,"h":800,"namespace":"omarchy-bar","pid":724}],
  "3":[{"address":"0x3","x":881,"y":0,"w":399,"h":800,"namespace":"omarchy-notifications","pid":724}]}},
 "S7NEG":{"levels":{
  "2":[{"address":"0x4","x":-1280,"y":-720,"w":1280,"h":720,"namespace":"omarchy-bar","pid":724}]}}
}"#;

#[test]
fn layers_cover_every_output_and_level() {
    let layers = parse_layers(LAYERS).unwrap_or_default();
    assert_eq!(layers.len(), 4);
    let bar = layers.iter().find(|layer| layer.output == "S7NEG").cloned();
    let bar = bar.map(|layer| (layer.namespace, layer.level, layer.rect.x, layer.pid));
    assert_eq!(bar, Some(("omarchy-bar".to_owned(), 2, -1280.0, Some(724))));
    assert!(
        layers
            .iter()
            .any(|layer| layer.level == 3 && layer.rect.width == 399.0)
    );
}

#[test]
fn an_unreadable_layer_makes_the_reply_unusable() {
    let body = br#"{"Virtual-1":{"levels":{"2":[{"x":0,"y":0,"namespace":"bar"}]}}}"#;
    assert_eq!(parse_layers(body), None);
    assert_eq!(
        parse_layers(br#"{"Virtual-1":{"levels":{"top":[]}}}"#),
        None
    );
}
