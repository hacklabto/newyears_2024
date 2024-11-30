use core::marker::PhantomData;
use embassy_rp::interrupt;
use embassy_rp::pwm::{ChannelAPin, ChannelBPin, Config, Pwm, Slice};
use embassy_rp::Peripheral;
use fixed::FixedU16;

const AUDIO_SIZE: usize = 1462987;
const AUDIO: &[u8; AUDIO_SIZE] = include_bytes!("../assets/ode.bin");

pub struct Pipe<T: Copy + Default, const N: usize> {
    buffer: [T; N],
    read_idx: usize,
    write_idx: usize,
}

impl<T: Copy + Default, const N: usize> Pipe<T, N> {
    pub fn new() -> Self {
        Self {
            buffer: [T::default(); N],
            read_idx: 0,
            write_idx: 0,
        }
    }

    pub fn add(&mut self, value: T) -> bool {
        let next_write_idx = (self.write_idx + 1) % N;
        if next_write_idx == self.read_idx {
            // Hit the read head - return false as "unable to add"
            return false;
        }
        self.buffer[self.write_idx] = value;
        self.write_idx = next_write_idx;
        return true;
    }

    pub fn get(&mut self) -> T {
        if self.read_idx == self.write_idx {
            T::default()
        } else {
            let rval = self.buffer[self.read_idx];
            self.read_idx = (self.read_idx + 1) % N;
            rval
        }
    }
}

const CONFIG_TOP: u16 = 512;
static mut SOUND_PIPE: Option<Pipe<u8, 48000>> = None;
static mut PWM_AB: Option<Pwm> = None;
static mut PWM_CONFIG: Option<Config> = None;
// target frequency is 48 khz
// PWM 125Mhz.  PWM resoltion is 8 bits, or 256.
// 125*1024*1024/256 (states)/48k
// = 10.6666666
//
const FRACTION_BITS_IN_CLOCK_DIVIDER: u16 = 16;
const RPI_FREQUENCY : u128 = 133000000;
//const TARGET_FREQUENCY : u128 = 48000;  bad squee sound
//const TARGET_FREQUENCY : u128 = 51853;  bad squee sound
//const TARGET_FREQUENCY : u128 = 43294;  bad squee sound
//const TARGET_FREQUENCY : u128 = 47500;  bad squee sound
const TARGET_FREQUENCY : u128 = 48800;    // cound sound, but why?
const CLOCK_DIVIDER : u128 = (FRACTION_BITS_IN_CLOCK_DIVIDER as u128) * RPI_FREQUENCY / (CONFIG_TOP as u128) / TARGET_FREQUENCY;
const CLOCK_DIVIDER_U16 : u16 = CLOCK_DIVIDER as u16;

pub struct Sound<PwmSlice: Slice> {
    //state: bool,
    //time_to_state_change: u32
    audio_pos: usize,
    // Add a dummy member so the struct can be tied to the PWM
    // interface being used
    pwm_device: PhantomData<PwmSlice>,
}

impl<PwmSlice: Slice> Sound<PwmSlice> {
    // These are hardware interfaces, so they will live for the entire program (+ 'static)
    pub fn new(
        pin_pos: impl Peripheral<P = impl ChannelAPin<PwmSlice>> + 'static,
        pin_neg: impl Peripheral<P = impl ChannelBPin<PwmSlice>> + 'static,
        pwm_slice: impl Peripheral<P = PwmSlice> + 'static,
    ) -> Self {
        let pwm_ab =
            embassy_rp::pwm::Pwm::new_output_ab(pwm_slice, pin_pos, pin_neg, Default::default());

        unsafe {
            PWM_AB = Some(pwm_ab);
            PWM_CONFIG = Some(Config::default());
            let config = PWM_CONFIG.as_mut().unwrap();
            config.top = CONFIG_TOP;
            config.compare_b = CONFIG_TOP / 2;
            config.divider = FixedU16::from_bits(CLOCK_DIVIDER_U16);
            config.invert_b = true;
            PWM_AB.as_mut().unwrap().set_config(config);
            SOUND_PIPE = Some(Pipe::new());
        }

        // Enable the interrupt for pwm slice 0
        embassy_rp::pac::PWM.inte().modify(|w| w.set_ch0(true));
        unsafe {
            cortex_m::peripheral::NVIC::unmask(interrupt::PWM_IRQ_WRAP);
        }

        Self {
            audio_pos: 0,
            pwm_device: PhantomData,
        }
    }

    pub fn update(&mut self) {
        unsafe {
            let mut try_to_add_next: bool = true;
            while try_to_add_next {
                let value: u8 = AUDIO[self.audio_pos / 2];
                let added = SOUND_PIPE.as_mut().unwrap().add(value);
                if added {
                    self.audio_pos = (self.audio_pos + 1) % (AUDIO_SIZE * 2);
                }
                try_to_add_next = added;
            }
        }
    }
}

#[interrupt]
fn PWM_IRQ_WRAP() {
    unsafe {
        let value = SOUND_PIPE.as_mut().unwrap().get();
        let config = PWM_CONFIG.as_mut().unwrap();
        config.compare_a = value as u16;
        config.compare_b = value as u16;

        let pwm = PWM_AB.as_mut().unwrap();
        pwm.set_config(&config);
        pwm.clear_wrapped();
    }
}
