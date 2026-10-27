//! A small multi-head MLP for the commission and deployment levels: 2-D input,
//! tanh or ReLU hidden layers, one sigmoid output per head, full-batch Adam.

use oo_base::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Act {
    Linear,
    Tanh,
    Relu,
}

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub x: [f32; 2],
    pub head: usize,
    /// Target probability: 0 / 1 for hard labels, anything between for soft ones.
    pub y: f32,
}

impl Sample {
    pub fn new(x: f32, y: f32, label: bool) -> Self {
        Self { x: [x, y], head: 0, y: if label { 1.0 } else { 0.0 } }
    }

    pub fn label(&self) -> bool {
        self.y > 0.5
    }
}

#[derive(Clone, Debug)]
pub struct Mlp {
    pub act: Act,
    pub sizes: Vec<usize>,
    /// (weights out×in, bias) per layer.
    pub layers: Vec<(Vec<f32>, Vec<f32>)>,
    /// Feed x² and y² as two extra inputs.
    pub squares: bool,
}

/// Adam moments; kept outside the net so training can pause at checkpoints.
pub struct Adam {
    m: Vec<Vec<f32>>,
    v: Vec<Vec<f32>>,
    t: i32,
}

impl Adam {
    pub fn new(net: &Mlp) -> Self {
        let m: Vec<Vec<f32>> = net.layers.iter().flat_map(|(w, b)| [vec![0.0; w.len()], vec![0.0; b.len()]]).collect();
        Self { v: m.clone(), m, t: 0 }
    }
}

impl Mlp {
    pub fn new(hidden: &[usize], heads: usize, act: Act, seed: u64) -> Self {
        Self::build(hidden, heads, act, seed, false)
    }

    pub fn build(hidden: &[usize], heads: usize, act: Act, seed: u64, squares: bool) -> Self {
        let mut rng = Rng::new(seed);
        let mut sizes = vec![if squares { 4 } else { 2 }];
        sizes.extend_from_slice(hidden);
        sizes.push(heads);
        let layers = sizes
            .windows(2)
            .map(|w| {
                let s = (1.0 / w[0] as f64).sqrt() * 1.4;
                ((0..w[0] * w[1]).map(|_| (rng.normal() * s) as f32).collect(), vec![0.0; w[1]])
            })
            .collect();
        Self { act, sizes, layers, squares }
    }

    pub fn params(&self) -> usize {
        self.layers.iter().map(|(w, b)| w.len() + b.len()).sum()
    }

    fn f(&self, z: f32) -> f32 {
        match self.act {
            Act::Linear => z,
            Act::Tanh => z.tanh(),
            Act::Relu => z.max(0.0),
        }
    }

    fn df(&self, z: f32) -> f32 {
        match self.act {
            Act::Linear => 1.0,
            Act::Tanh => 1.0 - z.tanh().powi(2),
            Act::Relu => {
                if z > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// Pre-activations and activations of every layer; the last is the sigmoid output.
    fn forward(&self, x: [f32; 2]) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
        let (a, b) = ((x[0] - 0.5) * 2.0, (x[1] - 0.5) * 2.0);
        let mut acts = vec![if self.squares { vec![a, b, a * a, b * b] } else { vec![a, b] }];
        let mut pre = Vec::new();
        let last = self.layers.len() - 1;
        for (l, (w, b)) in self.layers.iter().enumerate() {
            let (n_in, n_out) = (self.sizes[l], self.sizes[l + 1]);
            let input = acts.last().unwrap();
            let z: Vec<f32> = (0..n_out).map(|o| b[o] + (0..n_in).map(|i| w[o * n_in + i] * input[i]).sum::<f32>()).collect();
            let a = if l == last { z.iter().map(|&v| 1.0 / (1.0 + (-v).exp())).collect() } else { z.iter().map(|&v| self.f(v)).collect() };
            pre.push(z);
            acts.push(a);
        }
        (pre, acts)
    }

    /// Hidden activations of the last hidden layer (the shared trunk's output).
    pub fn features(&self, x: [f32; 2]) -> Vec<f32> {
        let (_, acts) = self.forward(x);
        acts[acts.len() - 2].clone()
    }

    pub fn prob(&self, x: [f32; 2], head: usize) -> f32 {
        self.forward(x).1.last().unwrap()[head]
    }

    /// Mean cross-entropy.
    pub fn loss(&self, data: &[Sample]) -> f32 {
        let n = data.len().max(1) as f32;
        data.iter()
            .map(|s| {
                let p = self.prob(s.x, s.head).clamp(1e-6, 1.0 - 1e-6);
                -(s.y * p.ln() + (1.0 - s.y) * (1.0 - p).ln())
            })
            .sum::<f32>()
            / n
    }

    pub fn accuracy(&self, data: &[Sample]) -> f32 {
        if data.is_empty() {
            return 0.0;
        }
        data.iter().filter(|s| (self.prob(s.x, s.head) > 0.5) == s.label()).count() as f32 / data.len() as f32
    }

    /// One full-batch Adam step on cross-entropy plus `l2`·|w|²/2; each sample trains only its head.
    pub fn epoch(&mut self, adam: &mut Adam, data: &[Sample], lr: f32, l2: f32) {
        if data.is_empty() {
            return;
        }
        let mut g: Vec<Vec<f32>> = adam.m.iter().map(|x| vec![0.0; x.len()]).collect();
        for s in data {
            let (pre, acts) = self.forward(s.x);
            let mut delta = vec![0.0; *self.sizes.last().unwrap()];
            delta[s.head] = acts.last().unwrap()[s.head] - s.y;
            for l in (0..self.layers.len()).rev() {
                let n_in = self.sizes[l];
                for (o, &d) in delta.iter().enumerate() {
                    if d == 0.0 {
                        continue;
                    }
                    for i in 0..n_in {
                        g[2 * l][o * n_in + i] += d * acts[l][i];
                    }
                    g[2 * l + 1][o] += d;
                }
                if l > 0 {
                    let w = &self.layers[l].0;
                    delta = (0..n_in)
                        .map(|i| delta.iter().enumerate().map(|(o, &d)| d * w[o * n_in + i]).sum::<f32>() * self.df(pre[l - 1][i]))
                        .collect();
                }
            }
        }
        adam.t += 1;
        let (b1, b2) = (0.9f32, 0.999f32);
        let (c1, c2) = (1.0 - b1.powi(adam.t), 1.0 - b2.powi(adam.t));
        for (k, grad) in g.iter().enumerate() {
            let (l, is_b) = (k / 2, k % 2 == 1);
            let target = if is_b { &mut self.layers[l].1 } else { &mut self.layers[l].0 };
            for (j, gj) in grad.iter().enumerate() {
                let gj = gj / data.len() as f32 + if is_b { 0.0 } else { l2 * target[j] };
                adam.m[k][j] = b1 * adam.m[k][j] + (1.0 - b1) * gj;
                adam.v[k][j] = b2 * adam.v[k][j] + (1.0 - b2) * gj * gj;
                target[j] -= lr * (adam.m[k][j] / c1) / ((adam.v[k][j] / c2).sqrt() + 1e-8);
            }
        }
    }

    pub fn train(&mut self, data: &[Sample], epochs: usize, lr: f32) {
        self.train_l2(data, epochs, lr, 0.0);
    }

    pub fn train_l2(&mut self, data: &[Sample], epochs: usize, lr: f32, l2: f32) {
        let mut adam = Adam::new(self);
        for _ in 0..epochs {
            self.epoch(&mut adam, data, lr, l2);
        }
    }
}

/// `n` points in the unit square, labelled by `rule`; points within `gap` of
/// the boundary (where `rule` flips under a small nudge) are skipped.
pub fn sample_square(seed: u64, n: usize, gap: f32, rule: impl Fn(f32, f32) -> bool) -> Vec<Sample> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let (x, y) = (rng.f32(), rng.f32());
        let l = rule(x, y);
        let near = [(gap, 0.0), (-gap, 0.0), (0.0, gap), (0.0, -gap)].iter().any(|&(dx, dy)| rule(x + dx, y + dy) != l);
        if !near {
            out.push(Sample::new(x, y, l));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learns_a_disk() {
        let rule = |x: f32, y: f32| (x - 0.5).powi(2) + (y - 0.5).powi(2) < 0.09;
        let train = sample_square(1, 160, 0.02, rule);
        let test = sample_square(2, 200, 0.02, rule);
        let mut net = Mlp::new(&[8], 1, Act::Tanh, 3);
        net.train(&train, 300, 0.05);
        assert!(net.accuracy(&test) > 0.93, "{}", net.accuracy(&test));
    }
}
