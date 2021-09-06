mod bytecode;

use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::mem::size_of;

use bytecode::Inst;

/// Enumeration Tag represents token types except for symbols such {, }, etc.
enum Tag {
    Id = 256,
    Num,
}

#[derive(Clone)]
struct TokenBase {
    tag: u32,
}

impl TokenBase {
    fn new(c: u32) -> TokenBase {
        TokenBase { tag: c }
    }
}

#[derive(Clone)]
struct WordBase {
    token: TokenBase,
    lexeme: String,
}

impl WordBase {
    #[allow(dead_code)]
    fn new(s: String, tag: u32) -> WordBase {
        WordBase {
            token: TokenBase::new(tag),
            lexeme: s,
        }
    }
}

impl PartialEq for WordBase {
    fn eq(&self, other: &Self) -> bool {
        if (*self).token.tag != (*other).token.tag {
            false;
        }
        (*self).lexeme == (*other).lexeme
    }
}

#[derive(Clone)]
struct Num {
    token: TokenBase,
    value: u32,
}

impl Num {
    fn new(v: u32) -> Num {
        Num {
            token: TokenBase {
                tag: Tag::Num as u32,
            },
            value: v,
        }
    }
}

#[derive(Clone)]
enum Token {
    Token(TokenBase),
    Word(WordBase),
    Num(Num),
    Eof,
}

impl Token {
    #[allow(dead_code)]
    fn new() -> Token {
        Token::Token(TokenBase::new(0))
    }

    #[allow(dead_code)]
    fn get_tag(&self) -> Option<u32> {
        match &*self {
            Token::Token(tok) => Some(tok.tag),
            Token::Word(word) => Some(word.token.tag),
            Token::Num(num) => Some(num.token.tag),
            Token::Eof => None,
        }
    }

    fn to_string(&self) -> String {
        match &*self {
            Token::Token(tok) => {
                let mut s = String::new();
                s.push(std::char::from_u32(tok.tag).unwrap());
                s
            }
            Token::Word(word) => word.lexeme.clone(),
            Token::Num(num) => format!("{}", num.value),
            _ => "Eof".to_string(),
        }
    }
}

struct Lexer {
    buf_reader: BufReader<File>,
    line_num: u32, // uses for syntax error reports
    peek: char,
    eof: bool,
}

impl Lexer {
    fn new(file_name: &str) -> Lexer {
        let lex = Lexer {
            buf_reader: BufReader::new(File::open(file_name).expect("open failed")),
            line_num: 1,
            peek: ' ',
            eof: false,
        };

        lex
    }

    fn read_char(&mut self) {
        let mut buffer = [0; 1];
        match self.buf_reader.read(&mut buffer) {
            Ok(x) => {
                if x != 0 {
                    self.peek = buffer[0] as char;
                } else {
                    self.eof = true;
                }
            }
            Err(_y) => panic!("read() failed{}", _y),
        };
    }

    fn scan(&mut self) -> Token {
        loop {
            if self.peek == ' ' || self.peek == '\t' {
                ()
            } else if self.peek == '\n' {
                self.line_num = self.line_num + 1;
            } else {
                break;
            }

            self.read_char();

            if self.eof {
                return Token::Eof;
            }
        }

        // Number handling
        if self.peek.is_digit(10) {
            let mut v: u32 = 0;
            loop {
                v = 10 * v + self.peek.to_digit(10).unwrap();
                self.read_char();
                if !self.peek.is_digit(10) {
                    break;
                }
            }
            if self.peek != '.' {
                return Token::Num(Num::new(v));
            }
        }

        // Word handle
        if self.peek.is_alphabetic() {
            let mut s = String::new();
            loop {
                s.push(self.peek);
                self.read_char();

                if !(self.peek.is_alphabetic() || self.peek.is_digit(10)) {
                    break;
                }
            }

            let w = WordBase {
                token: TokenBase {
                    tag: Tag::Id as u32,
                },
                lexeme: s.clone(),
            };
            return Token::Word(w);
        }

        let tok = Token::Token(TokenBase::new(self.peek as u32));
        self.peek = ' ';
        tok
    }
}

fn handle_reg(v: Token) -> u8 {
    let s = match v {
        Token::Word(word) => word.lexeme.clone(),
        _ => panic!("This token is not a Word"),
    };
    let mut num = String::new();

    for (pos, c) in s.char_indices() {
        if pos == 0 {
            assert!(c == 'v');
            continue;
        }
        assert!(c.is_digit(10));
        num.push(c);
    }
    num.parse::<u8>().unwrap()
}

fn handle_imm(imm: Token) -> u32 {
    match &imm {
        Token::Num(num) => num.value,
        _ => panic!("This token is not a Num, it is {}", imm.to_string()),
    }
}

struct Parser {
    lex: Lexer,
}

impl Parser {
    fn new(lex: Lexer) -> Parser {
        Parser { lex: lex }
    }

    fn match_(&mut self, expect: &str) {
        if &self.lex.scan().to_string() != expect {
            panic!("Token does not match the expected one");
        }
    }

    fn fetch_insts(&mut self) -> Vec<Inst> {
        let mut ret = Vec::new();
        let mut labels: HashMap<String, u32> = HashMap::new();

        loop {
            let mnemonic_token = self.lex.scan();
            let mnem = match &mnemonic_token {
                Token::Eof => break,
                _ => mnemonic_token.to_string(),
            };

            if mnem == "mov".to_string() {
                let v1 = self.lex.scan();
                self.match_(",");
                let v2 = self.lex.scan();

                ret.push(Inst::Mov(handle_reg(v1), handle_reg(v2)));
            } else if mnem == "movi" {
                let vr = self.lex.scan();
                self.match_(",");
                let imm = self.lex.scan();

                ret.push(Inst::Movi(handle_reg(vr), handle_imm(imm)));
            } else if mnem == "ldai".to_string() {
                let imm = self.lex.scan();

                ret.push(Inst::Ldai(handle_imm(imm)));
            } else if mnem == "lda" {
                let vr = self.lex.scan();

                ret.push(Inst::Lda(handle_reg(vr)));
            } else if mnem == "sta" {
                let vr = self.lex.scan();

                ret.push(Inst::Sta(handle_reg(vr)));
            } else if mnem == "add" {
                let vr = self.lex.scan();

                ret.push(Inst::Add(handle_reg(vr)));
            } else if mnem == "dec" {
                let vr = self.lex.scan();

                ret.push(Inst::Dec(handle_reg(vr)));
            } else if mnem == "bne" {
                let v1 = self.lex.scan();
                self.match_(",");
                let v2 = self.lex.scan();
                self.match_(",");
                let label = self.lex.scan().to_string();

                if !labels.contains_key(&label) {
                    panic!("Label {} not found", label);
                }

                ret.push(Inst::Bne(
                    handle_reg(v1),
                    handle_reg(v2),
                    *labels.get(&label).unwrap() as u32,
                ));
            } else if mnem == "print" {
                ret.push(Inst::Print);
            } else if mnem.starts_with("L") {
                labels.insert(mnem, ret.len() as u32);
                self.lex.scan();
            } else {
                panic!("Expected a mnemonic, got {}", mnem,);
            }
        }

        ret
    }
}

fn write_imm(file: &mut File, imm: u32) {
    let ptr = &imm as *const u32;
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, size_of::<u32>()) };
    assert_eq!(file.write(slice).unwrap(), size_of::<u32>());
}

fn write_inst(file: &mut File, inst: Inst) {
    let opcode = TryInto::<u8>::try_into(inst.clone()).unwrap();
    file.write(&[opcode]).unwrap();

    match inst {
        Inst::Mov(v1, v2) => {
            file.write(&[v1, v2]).unwrap();
        }

        Inst::Movi(v, imm) => {
            file.write(&[v]).unwrap();
            write_imm(file, imm);
        }
        Inst::Ldai(imm) => {
            write_imm(file, imm);
        }

        Inst::Lda(v) => {
            file.write(&[v]).unwrap();
        }
        Inst::Sta(v) => {
            file.write(&[v]).unwrap();
        }

        Inst::Add(v) => {
            file.write(&[v]).unwrap();
        }
        Inst::Dec(v) => {
            file.write(&[v]).unwrap();
        }

        Inst::Bne(v1, v2, imm) => {
            file.write(&[v1, v2]).unwrap();
            write_imm(file, imm);
        }
        Inst::Print => (),
    };
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Lexical analyzer needs 2 arguments - source file name and output file name");
        return ();
    }
    let lex = Lexer::new(&args[1]);
    let mut parser = Parser::new(lex);
    let instructions = parser.fetch_insts();

    let mut file = File::create(&args[2]).unwrap();
    for inst in instructions {
        write_inst(&mut file, inst);
    }
}
