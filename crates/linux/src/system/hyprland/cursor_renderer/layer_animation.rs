//! Hyprland 0.56 moves a mapped layer surface toward a new margin with its
//! `layersIn` animation, so the cursor is drawn between its old and new
//! point after the frame callback of its arrival has fired (probe row
//! L43-6). Before it maps a surface, the renderer asks its compositor for a
//! `no_anim` layer rule on the cursor namespace, so a margin change is drawn
//! where it is committed.

#[cfg(test)]
#[path = "layer_animation_tests.rs"]
mod tests;
