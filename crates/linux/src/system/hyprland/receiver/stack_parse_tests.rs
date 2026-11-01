use super::*;

const CLIENTS: &[u8] = br#"[
 {"address":"0x556a218cbb20","mapped":true,"hidden":false,"at":[12,38],"size":[1256,750],
  "workspace":{"id":1,"name":"1"},"floating":false,"monitor":0,"pid":3704,
  "pinned":false,"fullscreen":0},
 {"address":"0x556a218cbc40","mapped":true,"hidden":false,"at":[-1012,-602],"size":[300,200],
  "workspace":{"id":7,"name":"7"},"floating":true,"monitor":1,"pid":3705,
  "pinned":true,"fullscreen":2}
]"#;

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
fn clients_carry_their_stacking_state() {
    let clients = parse_stack_clients(CLIENTS).unwrap_or_default();
    assert_eq!(clients.len(), 2);
    assert_eq!(clients[0].address, 0x556a218cbb20);
    assert!(
        !clients[0].stacking.floating
            && !clients[0].stacking.pinned
            && !clients[0].stacking.fullscreen
    );
    assert!(clients[0].shown);
    assert_eq!(clients[1].rect.x, -1012.0);
    assert!(
        clients[1].stacking.floating
            && clients[1].stacking.pinned
            && clients[1].stacking.fullscreen
    );
    assert_eq!(clients[1].workspace.name, "7");
    assert_eq!(clients[1].workspace.monitor, Some(1));
}

#[test]
fn a_client_without_stacking_state_makes_the_reply_unusable() {
    let body = br#"[{"address":"0x1","at":[0,0],"size":[1,1],"pid":1,"mapped":true}]"#;
    assert_eq!(parse_stack_clients(body), None);
}

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
