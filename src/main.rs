use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::LinkedList;
use std::fs;
use std::io::{stdin, Cursor};
use std::vec::IntoIter;

const ADDRESS_RANGE: usize = 1 << 15;
const INTEGER_RANGE: usize = 1 << 15;
const MEMORY_SIZE: usize = 1 << 15;
const NUMBER_OF_REGISTERS: usize = 8;

struct Machine<'a> {
    memory: Box<Vec<u16>>,
    registers: Box<Vec<u16>>,
    stack: &'a mut LinkedList<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Instruction {
    Halt,
    Set(u16, u16),
    Push(u16),
    Pop(u16),
    Eq(u16, u16, u16),
    Gt(u16, u16, u16),
    Jmp(u16),
    Jt(u16, u16),
    Jf(u16, u16),
    Add(u16, u16, u16),
    Mult(u16, u16, u16),
    Mod(u16, u16, u16),
    And(u16, u16, u16),
    Or(u16, u16, u16),
    Not(u16, u16),
    Rmem(u16, u16),
    Wmem(u16, u16),
    Call(u16),
    Ret,
    Out(u16),
    In(u16),
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Address {
    Mem(usize),
    Reg(usize),
}

fn read_mem(mach: &mut Machine, address: Address) -> u16 {
    match address {
        Address::Mem(addr) => mach.memory[addr],
        Address::Reg(addr) => mach.registers[addr],
    }
}

fn write_mem(mach: &mut Machine, address: Address, value: u16) {
    match address {
        Address::Mem(addr) => mach.memory[addr] = value,
        Address::Reg(addr) => {
            mach.registers[addr] = value as u16;
        }
    }
}

fn get_oprnd_value(mut mach: &mut Machine, raw_addr: u16) -> u16 {
    let value: u16 = read_mem(&mut mach, get_addr(raw_addr).unwrap());
    if (value as usize) < INTEGER_RANGE {
        return value;
    } else {
        read_mem(&mut mach, get_addr(value).unwrap())
    }
}

fn get_op(mut mach: &mut Machine, raw_addr: u16) -> Option<Instruction> {
    let addr = get_addr(raw_addr).unwrap();
    let instr: u16 = read_mem(&mut mach, addr);
    match instr {
        0 | 18 | 21 => match instr {
            0 => Some(Instruction::Halt),
            18 => Some(Instruction::Ret),
            21 => Some(Instruction::Noop),
            _ => None,
        },
        2 | 3 | 6 | 17 | 19 | 20 => {
            let a_raw: u16 = read_mem(&mut mach, get_addr(raw_addr + 1).unwrap());
            let a: u16 = get_oprnd_value(&mut mach, raw_addr + 1);
            match instr {
                2 => Some(Instruction::Push(a)),
                3 => Some(Instruction::Pop(a_raw)),
                6 => Some(Instruction::Jmp(a)),
                17 => Some(Instruction::Call(a)),
                19 => Some(Instruction::Out(a)),
                20 => Some(Instruction::In(a_raw)),
                _ => None,
            }
        }
        1 | 7 | 8 | 14 | 15 | 16 => {
            let a_raw: u16 = read_mem(&mut mach, get_addr(raw_addr + 1).unwrap());
            let a: u16 = get_oprnd_value(&mut mach, raw_addr + 1);
            let b: u16 = get_oprnd_value(&mut mach, raw_addr + 2);
            match instr {
                1 => Some(Instruction::Set(a_raw, b)),
                7 => Some(Instruction::Jt(a, b)),
                8 => Some(Instruction::Jf(a, b)),
                14 => Some(Instruction::Not(a_raw, b)),
                15 => Some(Instruction::Rmem(a_raw, b)),
                16 => Some(Instruction::Wmem(a, b)),
                _ => None,
            }
        }
        4 | 5 | 9 | 10 | 11 | 12 | 13 => {
            let a_raw: u16 = read_mem(&mut mach, get_addr(raw_addr + 1).unwrap());
            let b: u16 = get_oprnd_value(&mut mach, raw_addr + 2);
            let c: u16 = get_oprnd_value(&mut mach, raw_addr + 3);
            match instr {
                4 => Some(Instruction::Eq(a_raw, b, c)),
                5 => Some(Instruction::Gt(a_raw, b, c)),
                9 => Some(Instruction::Add(a_raw, b, c)),
                10 => Some(Instruction::Mult(a_raw, b, c)),
                11 => Some(Instruction::Mod(a_raw, b, c)),
                12 => Some(Instruction::And(a_raw, b, c)),
                13 => Some(Instruction::Or(a_raw, b, c)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn get_addr(addr: u16) -> Option<Address> {
    if (addr as usize) < ADDRESS_RANGE {
        Some(Address::Mem(addr as usize))
    } else if (addr as usize) < ADDRESS_RANGE + NUMBER_OF_REGISTERS {
        Some(Address::Reg((addr as usize) - ADDRESS_RANGE))
    } else {
        None
    }
}

fn comp_op(mut mach: &mut Machine, instr: Instruction) {
    let raw_addr: u16;
    let value: u16;
    match instr {
        Instruction::Eq(a, b, c) => {
            raw_addr = a;
            if b == c {
                value = 1;
            } else {
                value = 0;
            }
        }
        Instruction::Gt(a, b, c) => {
            raw_addr = a;
            if b > c {
                value = 1;
            } else {
                value = 0;
            }
        }
        _ => return,
    }

    let addr: Address = get_addr(raw_addr).unwrap();
    write_mem(&mut mach, addr, value);
}

fn bin_op(mut mach: &mut Machine, op: fn(usize, usize) -> usize, instr: Instruction) {
    let addr: Address;
    let result: usize;
    match instr {
        Instruction::Add(a, b, c) | Instruction::Mult(a, b, c) => {
            addr = get_addr(a).unwrap();
            result = op(b as usize, c as usize) % INTEGER_RANGE
        }
        Instruction::Mod(a, b, c) | Instruction::And(a, b, c) | Instruction::Or(a, b, c) => {
            addr = get_addr(a).unwrap();
            result = op(b as usize, c as usize)
        }
        _ => return,
    }
    write_mem(&mut mach, addr, result as u16);
}

fn main() {
    let file = fs::read("challenge.bin").unwrap();

    let file_size = file.len() / 2;
    let mut buffer: [u16; MEMORY_SIZE] = [0; MEMORY_SIZE];
    let mut rdr: Cursor<Vec<u8>> = Cursor::new(file);
    rdr.read_u16_into::<LittleEndian>(&mut buffer[0..file_size])
        .unwrap();

    let stack: &mut LinkedList<u16> = &mut LinkedList::new();
    let mut machine: Machine = Machine {
        memory: Box::new(buffer.to_vec()),
        registers: Box::new(vec![0u16; NUMBER_OF_REGISTERS]),
        stack,
    };

    let mut input_iter: IntoIter<u8> = vec![].into_iter();

    let mut ip: u16 = 0;
    loop {
        let instr: Instruction = get_op(&mut machine, ip).unwrap();

        match instr {
            Instruction::Halt => break,
            Instruction::Set(a, b) => {
                let addr: Address = get_addr(a).unwrap();
                match addr {
                    Address::Reg(_) => {
                        write_mem(&mut machine, addr, b);
                        ip += 3;
                    }
                    _ => {
                        println!("Set operand is not an argument");
                    }
                }
            }
            Instruction::Push(a) => {
                machine.stack.push_front(a);
                ip += 2;
            }
            Instruction::Pop(a) => {
                let address = get_addr(a).unwrap();
                let value = machine.stack.pop_front().unwrap();
                write_mem(&mut machine, address, value);
                ip += 2;
            }
            Instruction::Eq(_, _, _) | Instruction::Gt(_, _, _) => {
                comp_op(&mut machine, instr);
                ip += 4;
            }
            Instruction::Jmp(a) => ip = a,
            Instruction::Jt(a, b) => {
                if a != 0 {
                    ip = b;
                } else {
                    ip += 3;
                }
            }
            Instruction::Jf(a, b) => {
                if a == 0 {
                    ip = b;
                } else {
                    ip += 3;
                }
            }
            Instruction::Add(_, _, _) => {
                bin_op(&mut machine, |x, y| x + y, instr);
                ip += 4;
            }
            Instruction::Mult(_, _, _) => {
                bin_op(&mut machine, |x, y| x * y, instr);
                ip += 4;
            }
            Instruction::Mod(_, _, _) => {
                bin_op(&mut machine, |x, y| x % y, instr);
                ip += 4;
            }
            Instruction::And(_, _, _) => {
                bin_op(&mut machine, |x, y| x & y, instr);
                ip += 4;
            }
            Instruction::Or(_, _, _) => {
                bin_op(&mut machine, |x, y| x | y, instr);
                ip += 4;
            }
            Instruction::Not(a, b) => {
                let addr: Address = get_addr(a).unwrap();
                let value: u16 = b ^ 0x7FFFu16;
                write_mem(&mut machine, addr, value);
                ip += 3;
            }
            Instruction::Rmem(a, b) => {
                let addr_a: Address = get_addr(a).unwrap();
                let addr_b: Address = get_addr(b).unwrap();
                let value: u16 = read_mem(&mut machine, addr_b);
                write_mem(&mut machine, addr_a, value);
                ip += 3;
            }
            Instruction::Wmem(a, b) => {
                let addr: Address = get_addr(a).unwrap();
                write_mem(&mut machine, addr, b);
                ip += 3;
            }
            Instruction::Call(a) => {
                machine.stack.push_front(ip + 2);
                ip = a;
            }
            Instruction::Ret => {
                let value = machine.stack.pop_front().unwrap();
                ip = value;
            }
            Instruction::Out(a) => {
                print!("{}", (a as u8) as char);
                ip += 2;
            }
            Instruction::In(a) => {
                if input_iter.len() == 0 {
                    let mut input = String::new();
                    stdin()
                        .read_line(&mut input)
                        .expect("Did not enter a correct string");

                    input_iter = input.into_bytes().into_iter();
                }

                write_mem(
                    &mut machine,
                    get_addr(a).unwrap(),
                    input_iter.next().unwrap() as u16,
                );

                ip += 2;
            }
            Instruction::Noop => ip += 1,
        }
    }
}
