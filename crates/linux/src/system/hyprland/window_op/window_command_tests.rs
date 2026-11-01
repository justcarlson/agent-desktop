use super::*;

const LUA: ConfigDialect = ConfigDialect::Lua;
const LEGACY: ConfigDialect = ConfigDialect::Legacy;

#[test]
fn lua_requests_aim_by_the_lower_case_address() {
    assert_eq!(
        move_command(LUA, 0x55D0A1, -50, 60),
        "dispatch hl.dsp.window.move({x=-50,y=60,window=\"address:0x55d0a1\"})"
    );
    assert_eq!(
        resize_command(LUA, 0x10, 500, 400),
        "dispatch hl.dsp.window.resize({x=500,y=400,window=\"address:0x10\"})"
    );
    assert_eq!(
        maximize_command(0x10),
        "dispatch hl.dsp.window.fullscreen({mode=\"maximized\",action=\"set\",window=\"address:0x10\"})"
    );
    assert_eq!(
        unset_fullscreen_command(0x10),
        "dispatch hl.dsp.window.fullscreen_state({internal=0,client=0,action=\"set\",window=\"address:0x10\"})"
    );
}

#[test]
fn legacy_requests_use_the_pixel_and_silent_dispatchers() {
    assert_eq!(
        move_command(LEGACY, 0x10, 40, 60),
        "dispatch movewindowpixel exact 40 60,address:0x10"
    );
    assert_eq!(
        resize_command(LEGACY, 0x10, 500, 400),
        "dispatch resizewindowpixel exact 500 400,address:0x10"
    );
    assert_eq!(
        workspace_command(LEGACY, 0x10, "special:agent-desktop-minimized-1"),
        "dispatch movetoworkspacesilent special:agent-desktop-minimized-1,address:0x10"
    );
}

#[test]
fn workspace_moves_never_follow_and_name_a_named_workspace() {
    assert_eq!(
        workspace_command(LUA, 0x10, "1"),
        "dispatch hl.dsp.window.move({workspace=\"1\",follow=false,window=\"address:0x10\"})"
    );
    assert_eq!(
        workspace_command(LUA, 0x10, "web"),
        "dispatch hl.dsp.window.move({workspace=\"name:web\",follow=false,window=\"address:0x10\"})"
    );
    assert_eq!(
        workspace_command(LEGACY, 0x10, "-3"),
        "dispatch movetoworkspacesilent name:-3,address:0x10"
    );
}
