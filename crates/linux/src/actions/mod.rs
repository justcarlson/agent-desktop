pub(crate) mod action_call;
pub(crate) mod chain;
pub(crate) mod chain_step;
pub(crate) mod click;
pub(crate) mod context;
pub(crate) mod disclosure;
pub(crate) mod dispatch;
pub(crate) mod focus;
pub(crate) mod mutation;
pub(crate) mod numeric_write;
pub(crate) mod physical_delivery;
mod process_check;
mod scroll_bus;
pub(crate) mod scroll_crawl;
pub(crate) mod scroll_find_bar;
mod scroll_into_view;
pub(crate) mod scroll_observe;
pub(crate) mod scroll_plan;
pub(crate) mod scroll_wrap;
pub(crate) mod secure_write;
pub(crate) mod select;
pub(crate) mod select_search;
pub(crate) mod text_write;
pub(crate) mod toggle_state;
pub(crate) mod value_write;
pub(crate) mod write_call;

pub(crate) use dispatch::execute_action;
pub(crate) use scroll_into_view::scroll_into_view;

#[cfg(test)]
mod action_scan_tests;
#[cfg(test)]
mod live_bench;
#[cfg(test)]
mod live_bench_env;
#[cfg(test)]
mod live_click_tests;
#[cfg(test)]
mod live_disclosure_tests;
#[cfg(test)]
mod live_dispatch_tests;
#[cfg(test)]
mod live_invariant_tests;
#[cfg(test)]
mod live_select_tests;
#[cfg(test)]
mod live_text_scripts;
#[cfg(test)]
mod live_text_tests;
#[cfg(test)]
mod live_toggle_tests;
#[cfg(test)]
mod live_value_tests;
#[cfg(test)]
mod live_widget_scripts;
#[cfg(test)]
mod live_widget_tests;
#[cfg(test)]
mod live_write_tests;
#[cfg(test)]
mod scroll_into_view_live_tests;
