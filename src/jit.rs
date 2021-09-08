use std::collections::BTreeSet;
use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use crate::bytecode;

/// Find first instructions in the basic blocks also known as "leaders"
pub fn find_leaders(bc: Vec<bytecode::Inst>) -> Vec<usize> {
    let mut leaders: Vec<usize> = Vec::new();

    if bc.is_empty() {
        // return an empty vector with no RawBlock's
        return Vec::new();
    }

    leaders.push(0);

    for i in 1..bc.len() {
        if bc[i].is_branch() {
            if let bytecode::Inst::Bne(_v1, _v2, imm) = bc[i] {
                leaders.push(imm as usize);
            }
            if i + 1 < bc.len() {
                leaders.push(i + 1);
            }
        } else {
            continue;
        }
    }

    #[cfg(debug_assertions)]
    {
        println!("The leaders:");
        for l in &leaders {
            println!("{}", l);
        }
    }

    leaders
}

struct SecondaryMap<K, V> {
    map: HashMap<K, V>,
    default: V,
}

impl<K, V> SecondaryMap<K, V>
where
    V: Default,
{
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            default: V::default(),
        }
    }

    fn clear(&mut self) {
        self.map.clear();
    }

    fn capacity(&self) -> usize {
        self.map.capacity()
    }
}

impl<K, V> Index<K> for SecondaryMap<K, V>
where
    K: Eq + std::hash::Hash,
    V: Default,
{
    type Output = V;

    #[inline(always)]
    fn index(&self, index: K) -> &V {
        self.map.get(&index).unwrap_or(&self.default)
    }
}

impl<K, V> IndexMut<K> for SecondaryMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Default,
{
    #[inline(always)]
    fn index_mut(&mut self, index: K) -> &mut V {
        if self.map.contains_key(&index) {
            self.map.get_mut(&index).unwrap_or(&mut self.default)
        } else {
            self.map.insert(index.clone(), V::default());
            self.map.get_mut(&index).unwrap()
        }
    }
}

type Block = u32;

#[derive(Default)]
struct BlockNode {
    prev: Option<Block>,
    next: Option<Block>,
    first_phi: Option<Inst>,
    first_inst: Option<Inst>,
    last_inst: Option<Inst>,
}

/// Iterate over blocks in layout order. See `Layout::blocks()`.
pub struct Blocks<'f> {
    layout: &'f Layout,
    next: Option<Block>,
}

impl<'f> Iterator for Blocks<'f> {
    type Item = Block;

    fn next(&mut self) -> Option<Block> {
        match self.next {
            Some(block) => {
                self.next = self.layout.next_block(block);
                Some(block)
            }
            None => None,
        }
    }
}

/// Use a layout reference in a for loop.
impl<'f> IntoIterator for &'f Layout {
    type Item = Block;
    type IntoIter = Blocks<'f>;

    fn into_iter(self) -> Blocks<'f> {
        self.blocks()
    }
}

type Inst = u32;

#[derive(Default)]
struct InstNode {
    block: Option<Block>,
    prev: Option<Inst>,
    next: Option<Inst>,
}

pub struct Layout {
    blocks: SecondaryMap<Block, BlockNode>,
    insts: SecondaryMap<Inst, InstNode>,
    first_block: Option<Block>,
    last_block: Option<Block>,
}

impl Layout {
    /// Create a new empty `Layout`.
    pub fn new() -> Self {
        Self {
            blocks: SecondaryMap::new(),
            insts: SecondaryMap::new(),
            first_block: None,
            last_block: None,
        }
    }

    /// Clear the layout.
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.insts.clear();
        self.first_block = None;
        self.last_block = None;
    }

    /// Returns the capacity of the `BlockData` map.
    pub fn block_capacity(&self) -> usize {
        self.blocks.capacity()
    }

    /// Is `block` currently part of the layout?
    pub fn is_block_inserted(&self, block: Block) -> bool {
        Some(block) == self.first_block || self.blocks[block].prev.is_some()
    }

    /// Insert `block` as the last block in the layout.
    pub fn append_block(&mut self, block: Block) {
        debug_assert!(
            !self.is_block_inserted(block),
            "Cannot append block that is already in the layout"
        );
        {
            let node = &mut self.blocks[block];
            debug_assert!(node.first_inst.is_none() && node.last_inst.is_none());
            node.prev = self.last_block.into();
            node.next = None.into();
        }

        if let Some(last) = self.last_block {
            self.blocks[last].next = block.into();
        } else {
            self.first_block = Some(block);
        }
        self.last_block = Some(block);
    }

    /// Get the block containing `inst`, or `None` if `inst` is not inserted in the layout.
    pub fn inst_block(&self, inst: Inst) -> Option<Block> {
        self.insts[inst].block.into()
    }

    /// Append `inst` to the end of `block`.
    pub fn append_inst(&mut self, inst: Inst, block: Block) {
        debug_assert_eq!(self.inst_block(inst), None);
        debug_assert!(
            self.is_block_inserted(block),
            "Cannot append instructions to block not in layout"
        );

        {
            let block_node = &mut self.blocks[block];
            {
                let inst_node = &mut self.insts[inst];
                inst_node.block = block.into();
                inst_node.prev = block_node.last_inst;
                debug_assert!(inst_node.next.is_none());
            }

            if block_node.first_inst.is_none() {
                block_node.first_inst = inst.into();
            } else {
                self.insts[block_node.last_inst.unwrap()].next = inst.into();
            }
            block_node.last_inst = inst.into();
        }
    }

    /// Return an iterator over all blocks in layout order.
    pub fn blocks(&self) -> Blocks {
        Blocks {
            layout: self,
            next: self.first_block,
        }
    }

    /// Get the block following `block` in the layout order.
    pub fn next_block(&self, block: Block) -> Option<Block> {
        self.blocks[block].next
    }
}

enum Opcode {
    Constant,
    Add,
    Sub,
    Bne,
    Phi,
}

enum InstData {
    Constant {
        opcode: Opcode,
        value: u32,
    },
    Binary {
        opcode: Opcode,
        inputs: [Inst; 2],
    },
    Bne {
        opcode: Opcode,
        inputs: [Inst; 2],
        succs: [Block; 2],
    },
    Phi {
        opcode: Opcode,
        inputs: Vec<Inst>,
    },
}

impl InstData {
    fn inputs(&self) -> Option<Vec<Inst>> {
        match self {
            Self::Constant { .. } => None,
            Self::Binary { opcode, inputs } => Some(vec![inputs[0], inputs[1]]),
            Self::Bne { opcode, inputs, .. } => Some(vec![inputs[0], inputs[1]]),
            Self::Phi { opcode, inputs } => Some(inputs.clone()),
        }
    }
}

struct DataFlowGraph {
    // Data about all of the instructions in the function, including opcodes and inputs. The
    // instructions in this map are not in program order.
    insts: HashMap<Inst, InstData>,

    // Users of instructions
    users: SecondaryMap<Inst, BTreeSet<Inst>>,
}

impl DataFlowGraph {
    fn new() -> Self {
        Self {
            insts: HashMap::new(),
            users: SecondaryMap::new(),
        }
    }

    fn make_inst(&mut self, data: InstData) -> Inst {
        let ret = (self.insts.len() + 1) as u32;
        if let Some(inputs) = data.inputs() {
            for input in inputs {
                self.users[input].insert(ret);
            }
        }

        self.insts.insert(ret, data);
        ret
    }
}

#[derive(Default)]
struct CFGNode {
    preds: BTreeSet<Block>,
    succs: BTreeSet<Block>,
}

struct Function {
    dfg: DataFlowGraph,
    layout: Layout,
    cfg: SecondaryMap<Block, CFGNode>,
}

impl Function {
    fn new() -> Self {
        Self {
            dfg: DataFlowGraph::new(),
            layout: Layout::new(),
            cfg: SecondaryMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::jit::DataFlowGraph;
    use crate::jit::InstData;
    use crate::jit::Layout;
    use crate::jit::Opcode;

    #[test]
    fn layout() {
        // Create a Layout object
        let mut layout = Layout::new();
        let block0 = 0;
        let block1 = 1;
        let inst = 0;

        // Append 2 blocks and 1 instruction to the first block
        layout.append_block(block0);
        layout.append_inst(inst, block0);
        layout.append_block(block1);

        // Check the presence of blocks
        assert!(layout.is_block_inserted(block0));
        assert!(layout.is_block_inserted(block1));
        assert!(!layout.is_block_inserted(block1 + 1));

        // Check the block's layout
        assert_eq!(layout.next_block(block0), Some(block1));

        // Check instruction's block reference
        assert_eq!(layout.inst_block(inst), Some(block0));
    }

    #[test]
    fn data_flow_graph() {
        let mut dfg = DataFlowGraph::new();

        let const1_data = InstData::Constant {
            opcode: Opcode::Constant,
            value: 0,
        };

        let const2_data = InstData::Constant {
            opcode: Opcode::Constant,
            value: 1,
        };

        let const1 = dfg.make_inst(const1_data);
        let const2 = dfg.make_inst(const2_data);

        let add_data = InstData::Binary {
            opcode: Opcode::Add,
            inputs: [const1, const2],
        };

        let add = dfg.make_inst(add_data);

        assert_eq!(const1, 1);
        assert_eq!(const2, 2);
        assert_eq!(add, 3);
    }
}
