#![macro_use]

#[allow(unused_macros)]
macro_rules! pin_trait {
    ($signal:ident, $instance:path $(, $mode:path)?) => {
        #[doc = concat!(stringify!($signal), " pin trait")]
        pub trait $signal<T: $instance $(, M: $mode)?>: crate::gpio::Pin {
            #[doc = concat!("Get the AF number needed to use this pin as ", stringify!($signal))]
            fn af_num(&self) -> u8;
        }
    };
}
