use x11rb::protocol::xproto::ImageOrder;

pub(crate) fn msb_first(order: ImageOrder) -> bool {
    order == ImageOrder::MSB_FIRST
}

#[cfg(test)]
#[path = "byte_order_tests.rs"]
mod tests;
