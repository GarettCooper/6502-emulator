mod opcodes;
mod address_modes;

use std::u8;
use address_modes::*;

//Declare some type alias for clarity's sake
type AddressModeFunction = fn(&mut MOS6502) -> (AddressModeValue, u8);
type OpcodeFunction = fn(&mut MOS6502, AddressModeValue) -> u8;

///The value that will be added to the stack pointer
const STACK_PAGE: u16 = 0x0100;
///The address that the program counter will be read from when a non-maskable interrupt request is made
const NMI_ADDRESS_LOCATION: u16 = 0xfffa;
///The address that the program counter will be read from when reset is called
const RESET_ADDRESS_LOCATION: u16 = 0xfffa;
///The address that the program counter will be read from when an interrupt request is made or BRK is called
const IRQ_ADDRESS_LOCATION: u16 = 0xfffe;

#[derive(Debug, PartialEq, Clone)]
pub struct MOS6502{
    //Registers
    accumulator: u8 ,
    x_register: u8,
    y_register: u8,
    program_counter: u16,
    stack_pointer: u8,
    status_register: u8,
    //Callbacks
    read: fn(u16) -> u8,
    write: fn(u16, u8),
    //Other
    ///The number of cycles before the next opcode is run
    remaining_cycles: u8
}

impl MOS6502{

    ///Creates a new MOS6502 emulation
    pub fn new(read_fn: fn(u16) -> u8, write_fn: fn(u16, u8)) -> MOS6502{
        MOS6502{
            accumulator: 0x00,
            x_register: 0x00,
            y_register: 0x00,
            program_counter: 0x0000,
            stack_pointer: 0xFD,
            status_register: 0x34,
            read: read_fn,
            write: write_fn,
            remaining_cycles: 0
        }
    }

    ///Sets the function that will be called when the processor writes to an address
    pub fn set_write_callback(&mut self, callback :fn(u16, u8)){
        self.write = callback;
    }

    ///Sets the function that will be called when the processor reads from an address
    pub fn set_read_callback(&mut self, callback :fn(u16) -> u8){
        self.read = callback;
    }

    ///Runs a processor cycle
    pub fn cycle(&mut self){

        if self.remaining_cycles == 0 {
            //pre-increment program counter
            self.program_counter += 1;
            let instruction = opcodes::OPCODE_TABLE[self.read(self.program_counter) as usize];
            let (address_mode_value, mut extra_cycles) = instruction.find_address(self);
            extra_cycles += instruction.execute_instruction(self, address_mode_value);
            self.remaining_cycles += extra_cycles + instruction.get_cycles();
        }
        self.remaining_cycles -= 1;
    }

    fn write(&self, address: u16, data: u8){
        (self.write)(address, data);
    }

    ///Wraps the read function provided passed at creation
    fn read(&self, address: u16) -> u8{
        return (self.read)(address);
    }

    ///Wrapper to return 16 bits from the read function instead of 8
    fn read_16(&self, address: u16) -> u16{
        //Remember little-endianness
        return ((self.read(address + 1) as u16) << 8) | self.read(address) as u16;
    }

    ///Wrapper to write 16 bits instead of 8
    fn write_16(&self, address: u16, data: u16){
        //Remember little-endianness
        self.write(address, (data >> 8) as u8);
        self.write(address, data as u8);
    }

    ///Pushes a byte onto the stack
    fn push_stack(&mut self, data: u8){
        self.write(STACK_PAGE + self.stack_pointer as u16, data);
        self.stack_pointer -= 1;
    }

    ///Pushes two bytes onto the stack
    fn push_stack_16(&mut self, data: u16){
        self.push_stack((data >> 8) as u8);
        self.push_stack(data as u8);
    }

    ///Pops a byte from the stack
    fn pop_stack(&mut self) -> u8{
        self.stack_pointer += 1;
        return self.read(STACK_PAGE + self.stack_pointer as u16);
    }

    ///Pops two bytes from the stack
    fn pop_stack_16(&mut self) -> u16{
        let lo = self.pop_stack() as u16;
        let hi = self.pop_stack() as u16;
        return (hi << 8) | lo;
    }

    fn set_flag(&mut self, flag: StatusFlag, value: bool){
        //Clear flag
        self.status_register &= !(flag as u8);
        //TODO: Work out a branch free method of doing this, possibly converting flag values to bit index
        if value {
            self.status_register |= flag as u8
        }
    }

    fn get_flag(&self, flag: StatusFlag) -> bool{
        return (self.status_register & flag as u8) > 0;
    }

    pub fn reset(&mut self){
        self.accumulator = 0x00;
        self.x_register = 0x00;
        self.y_register = 0x00;
        self.program_counter = 0x0000;
        self.stack_pointer = 0xFD;
        self.status_register = 0x34;
    }
}

#[derive(Copy, Clone)]
enum StatusFlag {
    Carry= 0b00000001,
    Zero = 0b00000010,
    InterruptDisable = 0b00000100,
    Decimal = 0b00001000,
    Break = 0b00110000,
    BreakIrq = 0b00100000,
    Overflow = 0b01000000,
    Negative = 0b10000000
}