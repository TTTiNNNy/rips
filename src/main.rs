use std::marker::PhantomData;
use std::{thread, time};
use std::collections::VecDeque;
use std::env::Args;
use std::ops::DerefMut;


use rand::Rng;

// struct Foo {
//     pub foo: Box<dyn Fn(usize) -> usize>,
// }
//
// impl Foo {
//     fn new(foo: impl Fn(usize) -> usize + 'static) -> Self {
//         let now = SystemTime::now();
//         now.elapsed().unwrap().as_micros();
//         Self { foo: Box::new(foo) }
//     }
// }

fn fn_plug() {}

struct IrqTwiWrite {}

struct IrqTwiRead {}

struct IrqSpiWrite {}

struct IrqSpiRead {}


struct CallbackHandler<T, ARGS> {
    phantom: PhantomData<T>,
    pub callback: Option<Box<dyn Fn(ARGS)>>,
}

impl<T, ARGS> CallbackHandler<T, ARGS> {
    fn fn_plug(){}
}

#[derive(Clone, Copy, PartialEq)]
pub enum PollStatus{
    Done,
    Process
}

struct PollHandler<T, ARGS> {
    phantom: PhantomData<T>,
    pub temp_data: ARGS,
    pub callback: Option<Box<dyn Fn(&mut ARGS) -> PollStatus>>,
}

// impl<T, ARGS> PollHandler<T, ARGS> {
//     fn fn_plug(_: ARGS) -> PollStatus {true}
// }

struct Executor<'a>{
    poll_elements: VecDeque<&'a mut dyn Poll>
}

impl <'a>Executor<'a>{
    fn poll_elements(&mut self)
    {
        for nb in 0..self.poll_elements.len() {
            if  (self.poll_elements[nb]).poll() == PollStatus::Done {self.poll_elements.remove(nb);}
        }
    }

    fn add(&mut self, new_item: &'a mut dyn Poll) {
        self.poll_elements.push_back(new_item);
    }

    fn remove_at(&mut self, index: usize) {
        self.poll_elements.remove(index);
    }

    fn is_empty(&self) -> bool{
        self.poll_elements.is_empty()
    }
}

pub trait Poll{
    fn poll(&mut self) -> PollStatus;
}

impl<T, ARGS> Poll for PollHandler<T, ARGS> {
    fn poll(&mut self) -> PollStatus {
        unsafe {
        let status = (self.callback.as_ref()).unwrap_unchecked()(&mut self.temp_data);
            status
        }

    }
}

pub trait AsyncWork<T, U> {
    fn then(&self, _: T);
}

enum IrqType {
    Write,
    Read,
}

static mut GLOBAL_TWI_WRITE: CallbackHandler<IrqTwiWrite, ()> = CallbackHandler::<IrqTwiWrite, ()> { callback: None, phantom: PhantomData {} };
static mut GLOBAL_TWI_READ: CallbackHandler<IrqTwiRead, usize> = CallbackHandler::<IrqTwiRead, usize> { callback: None, phantom: PhantomData {} };
static mut GLOBAL_FLASH_WRITE: PollHandler<IrqSpiWrite, usize> = PollHandler::<IrqSpiWrite, usize> { callback: None, phantom: PhantomData {}, temp_data: 0 };
static mut GLOBAL_EXECUTER: Executor = Executor{ poll_elements: VecDeque::new() };

unsafe impl<T, ARGS> Sync for CallbackHandler<T, ARGS> {}

/// in real apps and sdk it will looks like twi.interrupt.add(||<MACRO_NAME>(GLOBAL_TWI_READ));
/// or after bme280.read(); while(!bme280.is_ready){}; at thread.
fn twi_irq(irq_type: IrqType) {
    match irq_type {
        IrqType::Write => {}
        IrqType::Read => unsafe {
            let mut rng = rand::thread_rng();
            let val_from_dev = rng.gen_range(10..30);
            println!("callback bme read exec");
            ((&GLOBAL_TWI_READ).callback.as_ref().unwrap())(val_from_dev);
            (GLOBAL_TWI_READ).callback = None
        }
    }
}

// fn spi_irq(irq_type: IrqType) {
//     match irq_type {
//         IrqType::Write => unsafe {
//             println!("callback flash write exec");
//             ((&GLOBAL_FLASH_WRITE).callback.as_ref().unwrap())(());
//             (GLOBAL_FLASH_WRITE).callback = None
//         }
//         IrqType::Read => {}
//     }
// }

struct DriverBme280 {}

struct Bme280 {
    driver_bme_280: DriverBme280,
}


impl Bme280 {
    pub fn write(&self, func: &'static dyn Fn(())) {
        println!("set write bme callback ");
        <Bme280 as AsyncWork<&dyn Fn(()), IrqTwiWrite>>::then(self, func);
        self.driver_bme_280.write();
    }
    pub fn read(&self, func: &'static dyn Fn(usize)) {
        println!("set read bme callback ");
        <Bme280 as AsyncWork<&dyn Fn(usize), IrqTwiRead>>::then(self, func);
        self.driver_bme_280.read();
    }
}

impl DriverBme280 {
    pub fn write(&self) {
        println!("start bme driver block write");
        thread::sleep(time::Duration::from_secs(1));
        println!("end bme driver block write");
        twi_irq(IrqType::Write);
    }
    pub fn read(&self) {
        println!("start bme driver block read");
        thread::sleep(time::Duration::from_secs(1));
        println!("end bme driver block read");
        twi_irq(IrqType::Read);
    }
}

struct DriverFlash {}

impl DriverFlash {
    pub fn write() {
        println!("start flash driver dma write");
        thread::sleep(time::Duration::from_secs(1));
        println!("end flash driver block write");
        twi_irq(IrqType::Write);
    }
    pub fn read() {
        println!("start flash driver dma read");
        thread::sleep(time::Duration::from_secs(1));
        println!("end flash driver block read");
        twi_irq(IrqType::Read);
    }
}

struct Flash {
    driver_flash: DriverFlash,
}

impl Flash {
    pub fn write(&self, _str: String, func: &'static dyn Fn(&mut usize) -> PollStatus) {
        self.then(func);

        let thread_handle = thread::spawn(|| {
            DriverFlash::write();
            //spi_irq(IrqType::Write);
        }
        );
    }
    pub fn read(&self, func: &'static dyn Fn(())) {
        //self.then(func);
        thread::spawn(|| {
            DriverFlash::read();
            //spi_irq(IrqType::Read);
        });
    }
}

impl AsyncWork<&'static dyn Fn(usize), IrqTwiRead> for Bme280 {
    fn then(&self, func: &'static dyn Fn(usize)) {
        unsafe { (GLOBAL_TWI_READ).callback = Some(Box::new(func)); }
    }
}

impl <T>AsyncWork<&'static dyn Fn(()), T> for Bme280 {
    fn then(&self, func: &'static dyn Fn(())) {
        unsafe { (GLOBAL_TWI_WRITE).callback = Some(Box::new(func)); }
    }
}

// impl AsyncWork<&'static dyn Fn(()), IrqSpiWrite> for Flash {
//     fn then(&self, func: &'static dyn Fn(())) {
//         unsafe { (GLOBAL_FLASH_WRITE).callback = Some(Box::new(func)); }
//     }
// }

impl AsyncWork<&'static dyn Fn(&mut usize) -> PollStatus, IrqSpiWrite> for Flash {
    fn then(&self, func: &'static dyn Fn(&mut usize) -> PollStatus) {
        unsafe {
            (GLOBAL_FLASH_WRITE).callback = Some(Box::new(func));
            GLOBAL_EXECUTER.add(&mut GLOBAL_FLASH_WRITE);
        }
    }
}

fn func() {
    let bme280 = Bme280 { driver_bme_280: DriverBme280 {} };
    bme280.read(&|temp| {
        if temp > 10 {
            let flash = Flash { driver_flash: DriverFlash {} };
            flash.write("new entry: ".to_string() + (temp.to_string().as_str()), &|nb| {
                if *nb < 5 {
                    println!("New entry {}", nb);
                    *nb += 1;
                    return PollStatus::Process;
                }
                println!("Done");
                PollStatus::Done
            });
        }
    });
    //bme280().read().then(|temp: usize|{if temp > 20{flash.write("\n\rnew entry: ".push_str(temp.to_string())).then(uart.write("Done"));}});
}

fn main() {
    func();
    thread::sleep(time::Duration::from_secs(2));

    unsafe {
        while !GLOBAL_EXECUTER.is_empty() {
            GLOBAL_EXECUTER.poll_elements(); }
    }
}
