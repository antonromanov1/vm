mod bytecode;
mod jit;

use std::fs::File;
use std::io::Read;

use bytecode::Inst;
use jit::find_leaders;

fn fetch_insts(file: &mut File) -> Vec<Inst> {
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let mut iter = buffer.iter();
    let mut ret = Vec::new();

    loop {
        let opcode = match iter.next() {
            Some(byte) => *byte,
            None => break,
        };

        if opcode == 0 {
            let v1 = *iter.next().unwrap();
            let v2 = *iter.next().unwrap();
            ret.push(Inst::Mov(v1, v2));
        } else if opcode == 1 {
            let v = *iter.next().unwrap();
            let imm = &mut [0, 0, 0, 0];
            for i in 0..4 {
                imm[i] = *iter.next().unwrap();
            }

            ret.push(Inst::Movi(v, u32::from_le_bytes(*imm)));
        } else if opcode == 2 {
            let imm = &mut [0, 0, 0, 0];
            for i in 0..4 {
                imm[i] = *iter.next().unwrap();
            }

            ret.push(Inst::Ldai(u32::from_le_bytes(*imm)));
        } else if opcode == 3 {
            let v = *iter.next().unwrap();
            ret.push(Inst::Lda(v));
        } else if opcode == 4 {
            let v = *iter.next().unwrap();
            ret.push(Inst::Sta(v));
        } else if opcode == 5 {
            let v = *iter.next().unwrap();
            ret.push(Inst::Add(v));
        } else if opcode == 6 {
            let v = *iter.next().unwrap();
            ret.push(Inst::Dec(v));
        } else if opcode == 7 {
            let v1 = *iter.next().unwrap();
            let v2 = *iter.next().unwrap();
            let imm = &mut [0, 0, 0, 0];
            for i in 0..4 {
                imm[i] = *iter.next().unwrap();
            }

            ret.push(Inst::Bne(v1, v2, u32::from_le_bytes(*imm)));
        } else if opcode == 8 {
            ret.push(Inst::Print);
        } else {
            panic!("Invalid opcode: {}", opcode);
        }
    }

    ret
}

fn interpret(insts: Vec<Inst>) {
    let mut acc: u64 = 0;
    let mut regs: Vec<u64> = vec![0; 256];
    let mut i = 0;

    loop {
        if i == insts.len() {
            break;
        }

        match &insts[i] {
            Inst::Mov(v1, v2) => {
                regs[*v1 as usize] = regs[*v2 as usize];

                i = i + 1;
            }
            Inst::Movi(v, imm) => {
                regs[*v as usize] = *imm as u64;

                i = i + 1;
            }
            Inst::Ldai(imm) => {
                acc = *imm as u64;

                i = i + 1;
            }
            Inst::Lda(v) => {
                acc = regs[*v as usize];

                i = i + 1;
            }
            Inst::Sta(v) => {
                regs[*v as usize] = acc;

                i = i + 1;
            }
            Inst::Add(v) => {
                acc = acc + regs[*v as usize];

                i = i + 1;
            }
            Inst::Dec(v) => {
                regs[*v as usize] = regs[*v as usize] - 1;

                i = i + 1;
            }
            Inst::Bne(v1, v2, imm) => {
                if regs[*v1 as usize] != regs[*v2 as usize] {
                    i = *imm as usize;
                } else {
                    i = i + 1;
                }
            }
            Inst::Print => {
                println!("{}", acc);

                i = i + 1;
            }
        }
    }
}

fn main() {
    let _now = std::time::Instant::now();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Lexical analyzer needs 2 arguments - source file name and output file name");
        return ();
    }

    let mut file = File::open(&args[1]).unwrap();

    let insts = fetch_insts(&mut file);
    // interpret(insts);

    find_leaders(insts);

    // println!("Execution time: {} seconds", now.elapsed().as_secs());
}
