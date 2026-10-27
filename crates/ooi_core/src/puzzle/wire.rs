//! 连线谜题：把节点接成一条能工作的电路 / 流水线。
//!
//! 1.5 用 XOR 电路（两个隐藏节点分别取 OR / AND，输出相减），
//! 4.2 用层级流水线（必须按 边缘 → 角 → 形状 的顺序接）。

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Or,
    And,
}

impl Gate {
    pub fn label(self) -> &'static str {
        match self {
            Self::Or => "OR",
            Self::And => "AND",
        }
    }

    fn apply(self, bits: &[bool]) -> bool {
        if bits.is_empty() {
            return false;
        }
        match self {
            Self::Or => bits.iter().any(|b| *b),
            Self::And => bits.iter().all(|b| *b),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireTask {
    Xor,
    Chain,
}

#[derive(Clone, Debug)]
pub struct WireConfig {
    /// 节点世界坐标。
    pub positions: Vec<(f32, f32)>,
    /// 允许的连线，(from, to)。
    pub allowed: Vec<(usize, usize)>,
    /// 通关必须存在的连线。
    pub required: Vec<(usize, usize)>,
    pub task: WireTask,
}

#[derive(Clone, Debug)]
pub struct WirePuzzle {
    cfg: WireConfig,
    selected: BTreeSet<(usize, usize)>,
    gates: [Gate; 2],
}

impl WirePuzzle {
    pub fn new(cfg: WireConfig) -> Self {
        Self {
            cfg,
            selected: BTreeSet::new(),
            gates: [Gate::And, Gate::And],
        }
    }

    pub fn config(&self) -> &WireConfig {
        &self.cfg
    }

    pub fn selected(&self) -> &BTreeSet<(usize, usize)> {
        &self.selected
    }

    pub fn is_selected(&self, a: usize, b: usize) -> bool {
        self.selected.contains(&(a, b))
    }

    pub fn connect(&mut self, a: usize, b: usize) -> bool {
        if a == b || !self.cfg.allowed.contains(&(a, b)) {
            return false;
        }
        self.selected.insert((a, b));
        true
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    pub fn clear(&mut self) {
        self.selected.clear();
    }

    pub fn gates(&self) -> [Gate; 2] {
        self.gates
    }

    pub fn set_gate(&mut self, slot: usize, gate: Gate) {
        if slot < 2 {
            self.gates[slot] = gate;
        }
    }

    pub fn toggle_gate(&mut self, slot: usize) {
        if slot < 2 {
            self.gates[slot] = match self.gates[slot] {
                Gate::Or => Gate::And,
                Gate::And => Gate::Or,
            };
        }
    }

    fn hidden_output(&self, node: usize, gate: Gate, a: bool, b: bool) -> bool {
        let mut bits = Vec::new();
        if self.selected.contains(&(0, node)) {
            bits.push(a);
        }
        if self.selected.contains(&(1, node)) {
            bits.push(b);
        }
        gate.apply(&bits)
    }

    pub fn output(&self, a: bool, b: bool) -> bool {
        let h1 = self.hidden_output(2, self.gates[0], a, b);
        let h2 = self.hidden_output(3, self.gates[1], a, b);
        (h1 as i32 - h2 as i32) >= 1
    }

    pub fn truth_table(&self) -> [bool; 4] {
        [
            self.output(false, false),
            self.output(false, true),
            self.output(true, false),
            self.output(true, true),
        ]
    }

    pub fn is_solved(&self) -> bool {
        match self.cfg.task {
            WireTask::Chain => self
                .cfg
                .required
                .iter()
                .all(|edge| self.selected.contains(edge)),
            WireTask::Xor => {
                let wired = self
                    .cfg
                    .required
                    .iter()
                    .all(|edge| self.selected.contains(edge));
                wired && self.truth_table() == [false, true, true, false]
            }
        }
    }

    pub fn reset(&mut self) {
        self.selected.clear();
        self.gates = [Gate::And, Gate::And];
    }
}

pub fn xor_circuit() -> WireConfig {
    WireConfig {
        positions: vec![
            (-430.0, -120.0),
            (-430.0, 120.0),
            (-60.0, -120.0),
            (-60.0, 120.0),
            (360.0, 0.0),
        ],
        allowed: vec![(0, 2), (1, 2), (0, 3), (1, 3), (2, 4), (3, 4)],
        required: vec![(0, 2), (1, 2), (0, 3), (1, 3), (2, 4), (3, 4)],
        task: WireTask::Xor,
    }
}

pub fn hierarchy() -> WireConfig {
    WireConfig {
        positions: vec![
            (-520.0, 0.0),
            (-260.0, 70.0),
            (0.0, 0.0),
            (260.0, 70.0),
            (520.0, 0.0),
        ],
        allowed: vec![
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (0, 2),
            (1, 3),
            (2, 4),
            (0, 3),
        ],
        required: vec![(0, 1), (1, 2), (2, 3), (3, 4)],
        task: WireTask::Chain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wired() -> WirePuzzle {
        let mut p = WirePuzzle::new(xor_circuit());
        for (a, b) in [(0, 2), (1, 2), (0, 3), (1, 3), (2, 4), (3, 4)] {
            assert!(p.connect(a, b));
        }
        p
    }

    #[test]
    fn xor_needs_correct_gates() {
        let mut p = wired();
        assert!(!p.is_solved(), "AND/AND should not compute XOR");

        p.set_gate(0, Gate::Or);
        assert_eq!(p.truth_table(), [false, true, true, false]);
        assert!(p.is_solved(), "OR on top of AND is XOR");

        // 反过来：AND 减 OR 永远是负的。
        let mut q = wired();
        q.set_gate(0, Gate::And);
        q.set_gate(1, Gate::Or);
        assert!(!q.is_solved());
        assert_eq!(q.truth_table(), [false, false, false, false]);
    }

    #[test]
    fn xor_needs_all_wires() {
        let mut p = WirePuzzle::new(xor_circuit());
        p.set_gate(0, Gate::Or);
        p.set_gate(1, Gate::And);
        assert!(!p.is_solved());
        p.connect(0, 2);
        assert!(!p.is_solved());
    }

    #[test]
    fn rejects_illegal_edges() {
        let mut p = WirePuzzle::new(xor_circuit());
        assert!(!p.connect(0, 1));
        assert!(!p.connect(0, 4));
        assert!(!p.connect(2, 2));
    }

    #[test]
    fn chain_requires_order() {
        let mut p = WirePuzzle::new(hierarchy());
        assert!(!p.is_solved());
        p.connect(0, 2);
        p.connect(2, 4);
        assert!(!p.is_solved(), "shortcut must not count");
        for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 4)] {
            p.connect(a, b);
        }
        assert!(p.is_solved());
    }

    #[test]
    fn reset_clears_everything() {
        let mut p = wired();
        p.set_gate(0, Gate::Or);
        p.reset();
        assert_eq!(p.selected_count(), 0);
        assert_eq!(p.gates(), [Gate::And, Gate::And]);
    }
}
