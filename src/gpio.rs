//! General Purpose Input / Output

use core::convert::Infallible;
use core::marker::PhantomData;

use embedded_hal::digital::v2::{InputPin, OutputPin, StatefulOutputPin, ToggleableOutputPin};

use crate::pac::EXTI;
use crate::syscfg::SysCfg;

mod convert;

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self) -> Self::Parts;
}

pub trait PinExt {
    type Mode;
    /// Return pin number
    fn pin_id(&self) -> u8;
    /// Return port number
    fn port_id(&self) -> u8;
}

/// Some alternate mode (type state)
pub struct Alternate<const A: u8>;

/// Some alternate mode in open drain configuration (type state)
pub struct AlternateOD<const A: u8>;

// Compatibility constants
pub const AF0: u8 = 0;
pub const AF1: u8 = 1;
pub const AF2: u8 = 2;
pub const AF3: u8 = 3;
pub const AF4: u8 = 4;
pub const AF5: u8 = 5;
pub const AF6: u8 = 6;
pub const AF7: u8 = 7;
pub const AF8: u8 = 8;
pub const AF9: u8 = 9;
pub const AF10: u8 = 10;
pub const AF11: u8 = 11;
pub const AF12: u8 = 12;
pub const AF13: u8 = 13;
pub const AF14: u8 = 14;
pub const AF15: u8 = 15;

/// Input mode (type state)
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Floating input (type state)
pub struct Floating;

/// Pulled down input (type state)
pub struct PullDown;

/// Pulled up input (type state)
pub struct PullUp;

/// Open drain input or output (type state)
pub struct OpenDrain;

/// Output mode (type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Push pull output (type state)
pub struct PushPull;

/// Analog mode (type state)
pub struct Analog;

/// Digital output pin state
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PinState {
    /// Low pin state
    Low,
    /// High pin state
    High,
}

/// GPIO Pin speed selection
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Speed {
    Low = 0,
    Medium = 1,
    High = 2,
    VeryHigh = 3,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Edge {
    RISING,
    FALLING,
    RISING_FALLING,
}

mod sealed {
    /// Marker trait that show if `ExtiPin` can be implemented
    pub trait Interruptable {}
}

use sealed::Interruptable;
impl<MODE> Interruptable for Output<MODE> {}
impl<MODE> Interruptable for Input<MODE> {}

/// External Interrupt Pin
pub trait ExtiPin {
    fn make_interrupt_source(&mut self, syscfg: &mut SysCfg);
    fn trigger_on_edge(&mut self, exti: &mut EXTI, level: Edge);
    fn enable_interrupt(&mut self, exti: &mut EXTI);
    fn disable_interrupt(&mut self, exti: &mut EXTI);
    fn clear_interrupt_pending_bit(&mut self);
    fn check_interrupt(&self) -> bool;
}

impl<PIN> ExtiPin for PIN
where
    PIN: PinExt,
    PIN::Mode: Interruptable,
{
    /// Make corresponding EXTI line sensitive to this pin
    #[inline(always)]
    fn make_interrupt_source(&mut self, syscfg: &mut SysCfg) {
        let i = self.pin_id();
        let port = self.port_id() as u32;
        let offset = 4 * (i % 4);
        match i {
            0..=3 => {
                syscfg.exticr1.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (port << offset))
                });
            }
            4..=7 => {
                syscfg.exticr2.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (port << offset))
                });
            }
            8..=11 => {
                syscfg.exticr3.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (port << offset))
                });
            }
            12..=15 => {
                syscfg.exticr4.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (port << offset))
                });
            }
            _ => unreachable!(),
        }
    }

    /// Generate interrupt on rising edge, falling edge or both
    #[inline(always)]
    fn trigger_on_edge(&mut self, exti: &mut EXTI, edge: Edge) {
        let i = self.pin_id();
        match edge {
            Edge::RISING => {
                exti.rtsr
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << i)) });
                exti.ftsr
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << i)) });
            }
            Edge::FALLING => {
                exti.ftsr
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << i)) });
                exti.rtsr
                    .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << i)) });
            }
            Edge::RISING_FALLING => {
                exti.rtsr
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << i)) });
                exti.ftsr
                    .modify(|r, w| unsafe { w.bits(r.bits() | (1 << i)) });
            }
        }
    }

    /// Enable external interrupts from this pin.
    #[inline(always)]
    fn enable_interrupt(&mut self, exti: &mut EXTI) {
        exti.imr
            .modify(|r, w| unsafe { w.bits(r.bits() | (1 << self.pin_id())) });
    }

    /// Disable external interrupts from this pin
    #[inline(always)]
    fn disable_interrupt(&mut self, exti: &mut EXTI) {
        exti.imr
            .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << self.pin_id())) });
    }

    /// Clear the interrupt pending bit for this pin
    #[inline(always)]
    fn clear_interrupt_pending_bit(&mut self) {
        unsafe { (*EXTI::ptr()).pr.write(|w| w.bits(1 << self.pin_id())) };
    }

    /// Reads the interrupt pending bit for this pin
    #[inline(always)]
    fn check_interrupt(&self) -> bool {
        unsafe { ((*EXTI::ptr()).pr.read().bits() & (1 << self.pin_id())) != 0 }
    }
}

/// Partially erased pin
pub struct PXx<MODE, const P: char> {
    i: u8,
    _mode: PhantomData<MODE>,
}

impl<MODE, const P: char> PinExt for PXx<MODE, P> {
    type Mode = MODE;

    #[inline(always)]
    fn pin_id(&self) -> u8 {
        self.i
    }
    #[inline(always)]
    fn port_id(&self) -> u8 {
        P as u8 - 0x41
    }
}

impl<MODE, const P: char> PXx<Output<MODE>, P> {
    #[inline(always)]
    pub fn set_high(&mut self) {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { (*Gpio::<P>::ptr()).bsrr.write(|w| w.bits(1 << self.i)) }
    }

    #[inline(always)]
    pub fn set_low(&mut self) {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe {
            (*Gpio::<P>::ptr())
                .bsrr
                .write(|w| w.bits(1 << (self.i + 16)))
        }
    }

    #[inline(always)]
    pub fn get_state(&self) -> PinState {
        if self.is_set_low() {
            PinState::Low
        } else {
            PinState::High
        }
    }

    #[inline(always)]
    pub fn set_state(&mut self, state: PinState) {
        match state {
            PinState::Low => self.set_low(),
            PinState::High => self.set_high(),
        }
    }

    #[inline(always)]
    pub fn is_set_high(&self) -> bool {
        !self.is_set_low()
    }

    #[inline(always)]
    pub fn is_set_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*Gpio::<P>::ptr()).odr.read().bits() & (1 << self.i) == 0 }
    }

    #[inline(always)]
    pub fn toggle(&mut self) {
        if self.is_set_low() {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

impl<MODE, const P: char> OutputPin for PXx<Output<MODE>, P> {
    type Error = Infallible;

    #[inline(always)]
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high();
        Ok(())
    }

    #[inline(always)]
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low();
        Ok(())
    }
}

impl<MODE, const P: char> StatefulOutputPin for PXx<Output<MODE>, P> {
    #[inline(always)]
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_set_high())
    }

    #[inline(always)]
    fn is_set_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_set_low())
    }
}

impl<MODE, const P: char> ToggleableOutputPin for PXx<Output<MODE>, P> {
    type Error = Infallible;

    #[inline(always)]
    fn toggle(&mut self) -> Result<(), Self::Error> {
        self.toggle();
        Ok(())
    }
}

impl<const P: char> PXx<Output<OpenDrain>, P> {
    #[inline(always)]
    fn is_high(&self) -> bool {
        !self.is_low()
    }

    #[inline(always)]
    fn is_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*Gpio::<P>::ptr()).idr.read().bits() & (1 << self.i) == 0 }
    }
}

impl<const P: char> InputPin for PXx<Output<OpenDrain>, P> {
    type Error = Infallible;

    #[inline(always)]
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_high())
    }

    #[inline(always)]
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_low())
    }
}

impl<MODE, const P: char> PXx<Input<MODE>, P> {
    #[inline(always)]
    fn is_high(&self) -> bool {
        !self.is_low()
    }

    #[inline(always)]
    fn is_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*Gpio::<P>::ptr()).idr.read().bits() & (1 << self.i) == 0 }
    }
}

impl<MODE, const P: char> InputPin for PXx<Input<MODE>, P> {
    type Error = Infallible;

    #[inline(always)]
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_high())
    }

    #[inline(always)]
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_low())
    }
}

fn _set_alternate_mode<const P: char, const N: u8, const A: u8>() {
    let offset = 2 * { N };
    let offset2 = 4 * { N };
    let mode = A as u32;
    unsafe {
        if offset2 < 32 {
            (*Gpio::<P>::ptr())
                .afrl
                .modify(|r, w| w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2)));
        } else {
            let offset2 = offset2 - 32;
            (*Gpio::<P>::ptr())
                .afrh
                .modify(|r, w| w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2)));
        }
        (*Gpio::<P>::ptr())
            .moder
            .modify(|r, w| w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset)));
    }
}

/// Pin
pub struct PX<MODE, const P: char, const N: u8> {
    _mode: PhantomData<MODE>,
}
impl<MODE, const P: char, const N: u8> PX<MODE, P, N> {
    const fn new() -> Self {
        Self { _mode: PhantomData }
    }
}

impl<MODE, const P: char, const N: u8> PinExt for PX<MODE, P, N> {
    type Mode = MODE;

    #[inline(always)]
    fn pin_id(&self) -> u8 {
        N
    }
    #[inline(always)]
    fn port_id(&self) -> u8 {
        P as u8 - 0x41
    }
}

impl<MODE, const P: char, const N: u8> PX<Output<MODE>, P, N> {
    /// Set pin speed
    pub fn set_speed(self, speed: Speed) -> Self {
        let offset = 2 * { N };

        unsafe {
            (*Gpio::<P>::ptr())
                .ospeedr
                .modify(|r, w| w.bits((r.bits() & !(0b11 << offset)) | ((speed as u32) << offset)))
        };

        self
    }
}

impl<const P: char, const N: u8> PX<Output<OpenDrain>, P, N> {
    /// Enables / disables the internal pull up
    pub fn internal_pull_up(&mut self, on: bool) {
        let offset = 2 * { N };
        let value = if on { 0b01 } else { 0b00 };
        unsafe {
            (*Gpio::<P>::ptr())
                .pupdr
                .modify(|r, w| w.bits((r.bits() & !(0b11 << offset)) | (value << offset)))
        };
    }
}

impl<const P: char, const N: u8, const A: u8> PX<Alternate<A>, P, N> {
    /// Set pin speed
    pub fn set_speed(self, speed: Speed) -> Self {
        let offset = 2 * { N };

        unsafe {
            (*Gpio::<P>::ptr())
                .ospeedr
                .modify(|r, w| w.bits((r.bits() & !(0b11 << offset)) | ((speed as u32) << offset)))
        };

        self
    }

    /// Enables / disables the internal pull up
    pub fn internal_pull_up(self, on: bool) -> Self {
        let offset = 2 * { N };
        let value = if on { 0b01 } else { 0b00 };
        unsafe {
            (*Gpio::<P>::ptr())
                .pupdr
                .modify(|r, w| w.bits((r.bits() & !(0b11 << offset)) | (value << offset)))
        };

        self
    }
}

impl<const P: char, const N: u8, const A: u8> PX<Alternate<A>, P, N> {
    /// Turns pin alternate configuration pin into open drain
    pub fn set_open_drain(self) -> PX<AlternateOD<A>, P, N> {
        let offset = { N };
        unsafe {
            (*Gpio::<P>::ptr())
                .otyper
                .modify(|r, w| w.bits(r.bits() | (1 << offset)))
        };

        PX::new()
    }
}

impl<MODE, const P: char, const N: u8> PX<MODE, P, N> {
    /// Erases the pin number from the type
    ///
    /// This is useful when you want to collect the pins into an array where you
    /// need all the elements to have the same type
    pub fn downgrade(self) -> PXx<MODE, P> {
        PXx {
            i: { N },
            _mode: self._mode,
        }
    }

    /// Erases the pin number and the port from the type
    ///
    /// This is useful when you want to collect the pins into an array where you
    /// need all the elements to have the same type
    pub fn downgrade2(self) -> Pin<MODE> {
        Pin::new(P as u8 - 0x41, N)
    }
}

impl<MODE, const P: char, const N: u8> PX<Output<MODE>, P, N> {
    #[inline(always)]
    pub fn set_high(&mut self) {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { (*Gpio::<P>::ptr()).bsrr.write(|w| w.bits(1 << { N })) }
    }

    #[inline(always)]
    pub fn set_low(&mut self) {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe {
            (*Gpio::<P>::ptr())
                .bsrr
                .write(|w| w.bits(1 << ({ N } + 16)))
        }
    }

    #[inline(always)]
    pub fn get_state(&self) -> PinState {
        if self.is_set_low() {
            PinState::Low
        } else {
            PinState::High
        }
    }

    #[inline(always)]
    pub fn set_state(&mut self, state: PinState) {
        match state {
            PinState::Low => self.set_low(),
            PinState::High => self.set_high(),
        }
    }

    #[inline(always)]
    pub fn is_set_high(&self) -> bool {
        !self.is_set_low()
    }

    #[inline(always)]
    pub fn is_set_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*Gpio::<P>::ptr()).odr.read().bits() & (1 << { N }) == 0 }
    }

    #[inline(always)]
    pub fn toggle(&mut self) {
        if self.is_set_low() {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

impl<MODE, const P: char, const N: u8> OutputPin for PX<Output<MODE>, P, N> {
    type Error = Infallible;

    #[inline(always)]
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high();
        Ok(())
    }

    #[inline(always)]
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low();
        Ok(())
    }
}

impl<MODE, const P: char, const N: u8> StatefulOutputPin for PX<Output<MODE>, P, N> {
    #[inline(always)]
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_set_high())
    }

    #[inline(always)]
    fn is_set_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_set_low())
    }
}

impl<MODE, const P: char, const N: u8> ToggleableOutputPin for PX<Output<MODE>, P, N> {
    type Error = Infallible;

    #[inline(always)]
    fn toggle(&mut self) -> Result<(), Self::Error> {
        self.toggle();
        Ok(())
    }
}

impl<const P: char, const N: u8> PX<Output<OpenDrain>, P, N> {
    #[inline(always)]
    pub fn is_high(&self) -> bool {
        !self.is_low()
    }

    #[inline(always)]
    pub fn is_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*Gpio::<P>::ptr()).idr.read().bits() & (1 << { N }) == 0 }
    }
}

impl<const P: char, const N: u8> InputPin for PX<Output<OpenDrain>, P, N> {
    type Error = Infallible;

    #[inline(always)]
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_high())
    }

    #[inline(always)]
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_low())
    }
}

impl<MODE, const P: char, const N: u8> PX<Input<MODE>, P, N> {
    #[inline(always)]
    pub fn is_high(&self) -> bool {
        !self.is_low()
    }

    #[inline(always)]
    pub fn is_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*Gpio::<P>::ptr()).idr.read().bits() & (1 << { N }) == 0 }
    }
}

impl<MODE, const P: char, const N: u8> InputPin for PX<Input<MODE>, P, N> {
    type Error = Infallible;

    #[inline(always)]
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_high())
    }

    #[inline(always)]
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_low())
    }
}

macro_rules! gpio {
    ($GPIOX:ident, $gpiox:ident, $PXx:ident, $port_id:expr, $PXn:ident, [
        $($PXi:ident: ($pxi:ident, $i:expr, $MODE:ty),)+
    ]) => {
        /// GPIO
        pub mod $gpiox {
            use crate::pac::{$GPIOX, RCC};
            use crate::rcc::Enable;
            use super::{
                Floating, Input,
            };

            /// GPIO parts
            pub struct Parts {
                $(
                    /// Pin
                    pub $pxi: $PXi<$MODE>,
                )+
            }

            impl super::GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self) -> Parts {
                    unsafe {
                        // NOTE(unsafe) this reference will only be used for atomic writes with no side effects.
                        let rcc = &(*RCC::ptr());

                        // Enable clock.
                        $GPIOX::enable(rcc);
                    }
                    Parts {
                        $(
                            $pxi: $PXi::new(),
                        )+
                    }
                }
            }

            pub type $PXn<MODE> = super::PXx<MODE, $port_id>;

            $(
                pub type $PXi<MODE> = super::PX<MODE, $port_id, $i>;
            )+

        }
    }
}

gpio!(GPIOA, gpioa, PA, 'A', PAn, [
    PA0: (pa0, 0, Input<Floating>),
    PA1: (pa1, 1, Input<Floating>),
    PA2: (pa2, 2, Input<Floating>),
    PA3: (pa3, 3, Input<Floating>),
    PA4: (pa4, 4, Input<Floating>),
    PA5: (pa5, 5, Input<Floating>),
    PA6: (pa6, 6, Input<Floating>),
    PA7: (pa7, 7, Input<Floating>),
    PA8: (pa8, 8, Input<Floating>),
    PA9: (pa9, 9, Input<Floating>),
    PA10: (pa10, 10, Input<Floating>),
    PA11: (pa11, 11, Input<Floating>),
    PA12: (pa12, 12, Input<Floating>),
    PA13: (pa13, 13, Input<Floating>),
    PA14: (pa14, 14, Input<Floating>),
    PA15: (pa15, 15, Input<Floating>),
]);

gpio!(GPIOB, gpiob, PB, 'B', PBn, [
    PB0: (pb0, 0, Input<Floating>),
    PB1: (pb1, 1, Input<Floating>),
    PB2: (pb2, 2, Input<Floating>),
    PB3: (pb3, 3, Input<Floating>),
    PB4: (pb4, 4, Input<Floating>),
    PB5: (pb5, 5, Input<Floating>),
    PB6: (pb6, 6, Input<Floating>),
    PB7: (pb7, 7, Input<Floating>),
    PB8: (pb8, 8, Input<Floating>),
    PB9: (pb9, 9, Input<Floating>),
    PB10: (pb10, 10, Input<Floating>),
    PB11: (pb11, 11, Input<Floating>),
    PB12: (pb12, 12, Input<Floating>),
    PB13: (pb13, 13, Input<Floating>),
    PB14: (pb14, 14, Input<Floating>),
    PB15: (pb15, 15, Input<Floating>),
]);

gpio!(GPIOC, gpioc, PC, 'C', PCn, [
    PC0: (pc0, 0, Input<Floating>),
    PC1: (pc1, 1, Input<Floating>),
    PC2: (pc2, 2, Input<Floating>),
    PC3: (pc3, 3, Input<Floating>),
    PC4: (pc4, 4, Input<Floating>),
    PC5: (pc5, 5, Input<Floating>),
    PC6: (pc6, 6, Input<Floating>),
    PC7: (pc7, 7, Input<Floating>),
    PC8: (pc8, 8, Input<Floating>),
    PC9: (pc9, 9, Input<Floating>),
    PC10: (pc10, 10, Input<Floating>),
    PC11: (pc11, 11, Input<Floating>),
    PC12: (pc12, 12, Input<Floating>),
    PC13: (pc13, 13, Input<Floating>),
    PC14: (pc14, 14, Input<Floating>),
    PC15: (pc15, 15, Input<Floating>),
]);

#[cfg(feature = "gpiod")]
gpio!(GPIOD, gpiod, PD, 'D', PDn, [
    PD0: (pd0, 0, Input<Floating>),
    PD1: (pd1, 1, Input<Floating>),
    PD2: (pd2, 2, Input<Floating>),
    PD3: (pd3, 3, Input<Floating>),
    PD4: (pd4, 4, Input<Floating>),
    PD5: (pd5, 5, Input<Floating>),
    PD6: (pd6, 6, Input<Floating>),
    PD7: (pd7, 7, Input<Floating>),
    PD8: (pd8, 8, Input<Floating>),
    PD9: (pd9, 9, Input<Floating>),
    PD10: (pd10, 10, Input<Floating>),
    PD11: (pd11, 11, Input<Floating>),
    PD12: (pd12, 12, Input<Floating>),
    PD13: (pd13, 13, Input<Floating>),
    PD14: (pd14, 14, Input<Floating>),
    PD15: (pd15, 15, Input<Floating>),
]);

#[cfg(feature = "gpioe")]
gpio!(GPIOE, gpioe, PE, 'E', PEn, [
    PE0: (pe0, 0, Input<Floating>),
    PE1: (pe1, 1, Input<Floating>),
    PE2: (pe2, 2, Input<Floating>),
    PE3: (pe3, 3, Input<Floating>),
    PE4: (pe4, 4, Input<Floating>),
    PE5: (pe5, 5, Input<Floating>),
    PE6: (pe6, 6, Input<Floating>),
    PE7: (pe7, 7, Input<Floating>),
    PE8: (pe8, 8, Input<Floating>),
    PE9: (pe9, 9, Input<Floating>),
    PE10: (pe10, 10, Input<Floating>),
    PE11: (pe11, 11, Input<Floating>),
    PE12: (pe12, 12, Input<Floating>),
    PE13: (pe13, 13, Input<Floating>),
    PE14: (pe14, 14, Input<Floating>),
    PE15: (pe15, 15, Input<Floating>),
]);

#[cfg(feature = "gpiof")]
gpio!(GPIOF, gpiof, PF, 'F', PFn, [
    PF0: (pf0, 0, Input<Floating>),
    PF1: (pf1, 1, Input<Floating>),
    PF2: (pf2, 2, Input<Floating>),
    PF3: (pf3, 3, Input<Floating>),
    PF4: (pf4, 4, Input<Floating>),
    PF5: (pf5, 5, Input<Floating>),
    PF6: (pf6, 6, Input<Floating>),
    PF7: (pf7, 7, Input<Floating>),
    PF8: (pf8, 8, Input<Floating>),
    PF9: (pf9, 9, Input<Floating>),
    PF10: (pf10, 10, Input<Floating>),
    PF11: (pf11, 11, Input<Floating>),
    PF12: (pf12, 12, Input<Floating>),
    PF13: (pf13, 13, Input<Floating>),
    PF14: (pf14, 14, Input<Floating>),
    PF15: (pf15, 15, Input<Floating>),
]);

#[cfg(feature = "gpiog")]
gpio!(GPIOG, gpiog, PG, 'G', PGn, [
    PG0: (pg0, 0, Input<Floating>),
    PG1: (pg1, 1, Input<Floating>),
    PG2: (pg2, 2, Input<Floating>),
    PG3: (pg3, 3, Input<Floating>),
    PG4: (pg4, 4, Input<Floating>),
    PG5: (pg5, 5, Input<Floating>),
    PG6: (pg6, 6, Input<Floating>),
    PG7: (pg7, 7, Input<Floating>),
    PG8: (pg8, 8, Input<Floating>),
    PG9: (pg9, 9, Input<Floating>),
    PG10: (pg10, 10, Input<Floating>),
    PG11: (pg11, 11, Input<Floating>),
    PG12: (pg12, 12, Input<Floating>),
    PG13: (pg13, 13, Input<Floating>),
    PG14: (pg14, 14, Input<Floating>),
    PG15: (pg15, 15, Input<Floating>),
]);

#[cfg(not(feature = "stm32f401"))]
gpio!(GPIOH, gpioh, PH, 'H', PHn, [
    PH0: (ph0, 0, Input<Floating>),
    PH1: (ph1, 1, Input<Floating>),
    PH2: (ph2, 2, Input<Floating>),
    PH3: (ph3, 3, Input<Floating>),
    PH4: (ph4, 4, Input<Floating>),
    PH5: (ph5, 5, Input<Floating>),
    PH6: (ph6, 6, Input<Floating>),
    PH7: (ph7, 7, Input<Floating>),
    PH8: (ph8, 8, Input<Floating>),
    PH9: (ph9, 9, Input<Floating>),
    PH10: (ph10, 10, Input<Floating>),
    PH11: (ph11, 11, Input<Floating>),
    PH12: (ph12, 12, Input<Floating>),
    PH13: (ph13, 13, Input<Floating>),
    PH14: (ph14, 14, Input<Floating>),
    PH15: (ph15, 15, Input<Floating>),
]);

#[cfg(feature = "stm32f401")]
gpio!(GPIOH, gpioh, PH, 'H', PHn, [
    PH0: (ph0, 0, Input<Floating>),
    PH1: (ph1, 1, Input<Floating>),
]);

#[cfg(feature = "gpioi")]
gpio!(GPIOI, gpioi, PI, 'I', PIn, [
    PI0: (pi0, 0, Input<Floating>),
    PI1: (pi1, 1, Input<Floating>),
    PI2: (pi2, 2, Input<Floating>),
    PI3: (pi3, 3, Input<Floating>),
    PI4: (pi4, 4, Input<Floating>),
    PI5: (pi5, 5, Input<Floating>),
    PI6: (pi6, 6, Input<Floating>),
    PI7: (pi7, 7, Input<Floating>),
    PI8: (pi8, 8, Input<Floating>),
    PI9: (pi9, 9, Input<Floating>),
    PI10: (pi10, 10, Input<Floating>),
    PI11: (pi11, 11, Input<Floating>),
    PI12: (pi12, 12, Input<Floating>),
    PI13: (pi13, 13, Input<Floating>),
    PI14: (pi14, 14, Input<Floating>),
    PI15: (pi15, 15, Input<Floating>),
]);

#[cfg(feature = "gpioj")]
gpio!(GPIOJ, gpioj, PJ, 'J', PJn, [
    PJ0: (pj0, 0, Input<Floating>),
    PJ1: (pj1, 1, Input<Floating>),
    PJ2: (pj2, 2, Input<Floating>),
    PJ3: (pj3, 3, Input<Floating>),
    PJ4: (pj4, 4, Input<Floating>),
    PJ5: (pj5, 5, Input<Floating>),
    PJ6: (pj6, 6, Input<Floating>),
    PJ7: (pj7, 7, Input<Floating>),
    PJ8: (pj8, 8, Input<Floating>),
    PJ9: (pj9, 9, Input<Floating>),
    PJ10: (pj10, 10, Input<Floating>),
    PJ11: (pj11, 11, Input<Floating>),
    PJ12: (pj12, 12, Input<Floating>),
    PJ13: (pj13, 13, Input<Floating>),
    PJ14: (pj14, 14, Input<Floating>),
    PJ15: (pj15, 15, Input<Floating>),
]);

#[cfg(feature = "gpiok")]
gpio!(GPIOK, gpiok, PK, 'K', PKn, [
    PK0: (pk0, 0, Input<Floating>),
    PK1: (pk1, 1, Input<Floating>),
    PK2: (pk2, 2, Input<Floating>),
    PK3: (pk3, 3, Input<Floating>),
    PK4: (pk4, 4, Input<Floating>),
    PK5: (pk5, 5, Input<Floating>),
    PK6: (pk6, 6, Input<Floating>),
    PK7: (pk7, 7, Input<Floating>),
]);

/// Fully erased pin
pub struct Pin<MODE> {
    // Bits 0-3: Pin, Bits 4-7: Port
    pin_port: u8,
    _mode: PhantomData<MODE>,
}

impl<MODE> PinExt for Pin<MODE> {
    type Mode = MODE;

    #[inline(always)]
    fn pin_id(&self) -> u8 {
        self.pin_port & 0x0f
    }
    #[inline(always)]
    fn port_id(&self) -> u8 {
        self.pin_port >> 4
    }
}

impl<MODE> Pin<MODE> {
    fn new(port: u8, pin: u8) -> Self {
        Self {
            pin_port: port << 4 | pin,
            _mode: PhantomData,
        }
    }

    #[inline]
    fn block(&self) -> &crate::pac::gpioa::RegisterBlock {
        // This function uses pointer arithmetic instead of branching to be more efficient

        // The logic relies on the following assumptions:
        // - GPIOA register is available on all chips
        // - all gpio register blocks have the same layout
        // - consecutive gpio register blocks have the same offset between them, namely 0x0400
        // - Pin::new was called with a valid port

        // FIXME could be calculated after const_raw_ptr_to_usize_cast stabilization #51910
        const GPIO_REGISTER_OFFSET: usize = 0x0400;

        let offset = GPIO_REGISTER_OFFSET * self.port_id() as usize;
        let block_ptr =
            (crate::pac::GPIOA::ptr() as usize + offset) as *const crate::pac::gpioa::RegisterBlock;

        unsafe { &*block_ptr }
    }
}

impl<MODE> Pin<Output<MODE>> {
    #[inline(always)]
    pub fn set_high(&mut self) {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { self.block().bsrr.write(|w| w.bits(1 << self.pin_id())) };
    }

    #[inline(always)]
    pub fn set_low(&mut self) {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe {
            self.block()
                .bsrr
                .write(|w| w.bits(1 << (self.pin_id() + 16)))
        };
    }

    #[inline(always)]
    pub fn get_state(&self) -> PinState {
        if self.is_set_low() {
            PinState::Low
        } else {
            PinState::High
        }
    }

    #[inline(always)]
    pub fn set_state(&mut self, state: PinState) {
        match state {
            PinState::Low => self.set_low(),
            PinState::High => self.set_high(),
        }
    }

    #[inline(always)]
    pub fn is_set_high(&self) -> bool {
        !self.is_set_low()
    }

    #[inline(always)]
    pub fn is_set_low(&self) -> bool {
        self.block().odr.read().bits() & (1 << self.pin_id()) == 0
    }

    #[inline(always)]
    pub fn toggle(&mut self) {
        if self.is_set_low() {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

impl<MODE> OutputPin for Pin<Output<MODE>> {
    type Error = core::convert::Infallible;

    #[inline(always)]
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high();
        Ok(())
    }

    #[inline(always)]
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low();
        Ok(())
    }
}

impl<MODE> StatefulOutputPin for Pin<Output<MODE>> {
    #[inline(always)]
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_set_high())
    }

    #[inline(always)]
    fn is_set_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_set_low())
    }
}

impl<MODE> ToggleableOutputPin for Pin<Output<MODE>> {
    type Error = Infallible;

    #[inline(always)]
    fn toggle(&mut self) -> Result<(), Self::Error> {
        self.toggle();
        Ok(())
    }
}

impl Pin<Output<OpenDrain>> {
    #[inline(always)]
    fn is_high(&self) -> bool {
        !self.is_low()
    }

    #[inline(always)]
    fn is_low(&self) -> bool {
        self.block().idr.read().bits() & (1 << self.pin_id()) == 0
    }
}

impl InputPin for Pin<Output<OpenDrain>> {
    type Error = core::convert::Infallible;

    #[inline(always)]
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_high())
    }

    #[inline(always)]
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_low())
    }
}

impl<MODE> Pin<Input<MODE>> {
    #[inline(always)]
    fn is_high(&self) -> bool {
        !self.is_low()
    }

    #[inline(always)]
    fn is_low(&self) -> bool {
        self.block().idr.read().bits() & (1 << self.pin_id()) == 0
    }
}

impl<MODE> InputPin for Pin<Input<MODE>> {
    type Error = core::convert::Infallible;

    #[inline(always)]
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_high())
    }

    #[inline(always)]
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(self.is_low())
    }
}

struct Gpio<const P: char>;
impl<const P: char> Gpio<P> {
    const fn ptr() -> *const crate::pac::gpioa::RegisterBlock {
        match P {
            'A' => crate::pac::GPIOA::ptr(),
            'B' => crate::pac::GPIOB::ptr() as _,
            'C' => crate::pac::GPIOC::ptr() as _,
            #[cfg(feature = "gpiod")]
            'D' => crate::pac::GPIOD::ptr() as _,
            #[cfg(feature = "gpioe")]
            'E' => crate::pac::GPIOE::ptr() as _,
            #[cfg(feature = "gpiof")]
            'F' => crate::pac::GPIOF::ptr() as _,
            #[cfg(feature = "gpiog")]
            'G' => crate::pac::GPIOG::ptr() as _,
            'H' => crate::pac::GPIOH::ptr() as _,
            #[cfg(feature = "gpioi")]
            'I' => crate::pac::GPIOI::ptr() as _,
            #[cfg(feature = "gpioj")]
            'J' => crate::pac::GPIOJ::ptr() as _,
            #[cfg(feature = "gpiok")]
            'K' => crate::pac::GPIOK::ptr() as _,
            _ => crate::pac::GPIOA::ptr(),
        }
    }
}

/// Const assert hack
struct Assert<const L: u8, const R: u8>;

impl<const L: u8, const R: u8> Assert<L, R> {
    pub const LESS: u8 = R - L - 1;
}
