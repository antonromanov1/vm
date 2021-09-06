use std::convert::TryInto;

type Reg = u8;

#[derive(Clone, Copy)]
pub enum Inst {
    Mov(Reg, Reg),
    Movi(Reg, u32),
    Ldai(u32),
    Lda(Reg),
    Sta(Reg),
    Add(Reg),
    Dec(Reg),
    Bne(Reg, Reg, u32),
    Print,
}

impl Inst {
    pub fn is_branch(&self) -> bool {
        match self {
            Self::Bne { .. } => true,
            _ => false,
        }
    }
}

impl TryInto<u8> for Inst {
    type Error = ();

    fn try_into(self) -> Result<u8, Self::Error> {
        match &self {
            Inst::Mov(_, _) => Ok(0),
            Inst::Movi(_, _) => Ok(1),
            Inst::Ldai(_) => Ok(2),
            Inst::Lda(_) => Ok(3),
            Inst::Sta(_) => Ok(4),
            Inst::Add(_) => Ok(5),
            Inst::Dec(_) => Ok(6),
            Inst::Bne(_, _, _) => Ok(7),
            Inst::Print => Ok(8),
        }
    }
}
