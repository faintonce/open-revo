pub use openrevo_core::driver::streamer::tracker::*;

#[cfg(test)]
mod tests {
    #[test]
    fn regression_host_and_core_share_the_keyboard_event_channel() {
        let receiver = openrevo_core::driver::streamer::tracker::register_key_event_receiver();
        super::set_interactive_mode(true);
        super::push_interactive_key_event(42);
        assert_eq!(receiver.try_recv().unwrap(), 42);
        super::unregister_key_event_sender();
    }
}
