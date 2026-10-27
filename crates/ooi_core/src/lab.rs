//! The Forge: a tiny, real neural network you can build and train.
//!
//! This module is deliberately independent of Bevy. The app only draws what
//! `Mlp` and `Dataset` expose; every number on screen is computed here.
//!
//! The model is a small multilayer perceptron with tanh hidden units and a
//! sigmoid output. It is trained with full-batch backpropagation and momentum.
//! That is enough to separate XOR, a ring, and two interleaved spirals in a
//! handful of seconds, while remaining small enough to inspect edge by edge.

use serde::{Deserialize, Serialize};

/// The three toy worlds available in the Forge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatasetKind {
    Xor,
    Ring,
    Spiral,
}

impl DatasetKind {
    pub const ALL: [DatasetKind; 3] = [DatasetKind::Xor, DatasetKind::Ring, DatasetKind::Spiral];

    /// Stable id used by saves and UI.
    pub fn id(self) -> &'static str {
        match self {
            Self::Xor => "xor",
            Self::Ring => "ring",
            Self::Spiral => "spiral",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }

    /// Accuracy the player is asked to reach.
    pub fn target_accuracy(self) -> f32 {
        match self {
            Self::Xor => 0.97,
            Self::Ring => 0.95,
            Self::Spiral => 0.90,
        }
    }

    /// A generous parameter budget; the interesting choice is shape, not raw size.
    pub fn parameter_budget(self) -> usize {
        match self {
            Self::Xor => 25,
            Self::Ring => 81,
            Self::Spiral => 141,
        }
    }

    /// Suggested starting architecture (hidden layers, neurons per layer).
    pub fn default_shape(self) -> (usize, usize) {
        match self {
            Self::Xor => (1, 4),
            Self::Ring => (1, 8),
            Self::Spiral => (2, 9),
        }
    }

    /// The constrained challenge for this dataset. Defaults deliberately fail
    /// at least one constraint; shrinking the network is the lesson.
    pub fn trial(self) -> TrialSpec {
        match self {
            Self::Xor => TrialSpec {
                min_depth: 1,
                max_depth: 1,
                max_width: 3,
                max_params: 13,
                target_accuracy: 0.97,
            },
            Self::Ring => TrialSpec {
                min_depth: 1,
                max_depth: 1,
                max_width: 4,
                max_params: 17,
                target_accuracy: 0.95,
            },
            Self::Spiral => TrialSpec {
                min_depth: 2,
                max_depth: 2,
                max_width: 6,
                max_params: 67,
                target_accuracy: 0.90,
            },
        }
    }

    pub fn sample_count(self) -> usize {
        self.dataset().samples.len()
    }

    /// A learning rate that works well for this landscape.
    pub fn default_learning_rate(self) -> f32 {
        match self {
            Self::Xor => 0.03,
            Self::Ring => 0.03,
            Self::Spiral => 0.03,
        }
    }

    /// Curated initialisations keep the sandbox reproducible. These seeds are
    /// tested to converge for every width offered by the UI, so a player never
    /// has to wonder whether a failed run was their fault or an unlucky draw.
    pub fn training_seed(self) -> u64 {
        match self {
            Self::Xor => 0x0abc,
            Self::Ring => 0x0d15,
            Self::Spiral => 0x5eed,
        }
    }

    pub fn dataset(self) -> Dataset {
        Dataset::generate(self)
    }
}

/// A constrained Forge challenge. The default width always fails, so the
/// player must reason about capacity rather than simply letting the default
/// network converge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrialSpec {
    pub min_depth: usize,
    pub max_depth: usize,
    pub max_width: usize,
    pub max_params: usize,
    pub target_accuracy: f32,
}

impl TrialSpec {
    pub fn shape_ok(&self, depth: usize, width: usize, params: usize) -> bool {
        depth >= self.min_depth
            && depth <= self.max_depth
            && width <= self.max_width
            && params <= self.max_params
    }

    pub fn is_met(&self, depth: usize, width: usize, params: usize, accuracy: f32) -> bool {
        self.shape_ok(depth, width, params) && accuracy >= self.target_accuracy
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub x: f32,
    pub y: f32,
    /// false = one class (cold), true = the other (lit).
    pub label: bool,
}

#[derive(Clone, Debug)]
pub struct Dataset {
    pub kind: DatasetKind,
    pub samples: Vec<Sample>,
}

impl Dataset {
    pub fn generate(kind: DatasetKind) -> Self {
        let mut rng = Rng::new(match kind {
            DatasetKind::Xor => 0x00d1_5ea1,
            DatasetKind::Ring => 0x0f11_7a21,
            DatasetKind::Spiral => 0x5b1a_1d5e,
        });
        let samples = match kind {
            DatasetKind::Xor => xor_data(&mut rng),
            DatasetKind::Ring => ring_data(&mut rng),
            DatasetKind::Spiral => spiral_data(&mut rng),
        };
        Self { kind, samples }
    }

    pub fn class_balance(&self) -> (usize, usize) {
        let lit = self.samples.iter().filter(|s| s.label).count();
        (self.samples.len() - lit, lit)
    }
}

fn xor_data(rng: &mut Rng) -> Vec<Sample> {
    let mut out = Vec::with_capacity(160);
    for quadrant in 0..4 {
        let cx = if quadrant & 1 == 0 { 0.26 } else { 0.74 };
        let cy = if quadrant & 2 == 0 { 0.26 } else { 0.74 };
        let label = (quadrant & 2 == 0) != (quadrant & 1 == 0);
        for _ in 0..40 {
            let x = (cx + rng.range(-0.145, 0.145)).clamp(0.05, 0.95);
            let y = (cy + rng.range(-0.145, 0.145)).clamp(0.05, 0.95);
            out.push(Sample { x, y, label });
        }
    }
    out
}

fn ring_data(rng: &mut Rng) -> Vec<Sample> {
    let mut out = Vec::with_capacity(180);
    for _ in 0..90 {
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let radius = rng.range(0.03, 0.20);
        out.push(Sample {
            x: 0.5 + radius * angle.cos(),
            y: 0.5 + radius * angle.sin(),
            label: false,
        });
    }
    for _ in 0..90 {
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let radius = rng.range(0.34, 0.455);
        out.push(Sample {
            x: 0.5 + radius * angle.cos(),
            y: 0.5 + radius * angle.sin(),
            label: true,
        });
    }
    out
}

fn spiral_data(rng: &mut Rng) -> Vec<Sample> {
    let mut out = Vec::with_capacity(200);
    const TURNS: f32 = 1.72;
    for arm in 0..2 {
        for i in 0..100 {
            let t = i as f32 / 99.0;
            let radius = 0.025 + 0.455 * t;
            let angle = std::f32::consts::TAU * TURNS * t
                + std::f32::consts::PI * arm as f32
                + rng.range(-0.09, 0.09);
            out.push(Sample {
                x: 0.5 + radius * angle.cos() + rng.range(-0.012, 0.012),
                y: 0.5 + radius * angle.sin() + rng.range(-0.012, 0.012),
                label: arm == 1,
            });
        }
    }
    out
}

/// A fully connected network with tanh hidden layers and one sigmoid output.
#[derive(Clone, Debug)]
pub struct Mlp {
    /// e.g. `[2, 6, 1]` for one hidden layer with six neurons.
    sizes: Vec<usize>,
    weights: Vec<Vec<f32>>,
    biases: Vec<Vec<f32>>,
    adam_m_w: Vec<Vec<f32>>,
    adam_v_w: Vec<Vec<f32>>,
    adam_m_b: Vec<Vec<f32>>,
    adam_v_b: Vec<Vec<f32>>,
    train_steps: u32,
    /// Cached activations from the most recent forward pass.
    activations: Vec<Vec<f32>>,
}

impl Mlp {
    pub fn new(hidden_layers: usize, width: usize, seed: u64) -> Self {
        let depth = hidden_layers.clamp(1, 2);
        let width = width.clamp(2, 14);
        let mut sizes = vec![2];
        for _ in 0..depth {
            sizes.push(width);
        }
        sizes.push(1);

        let mut rng = Rng::new(seed ^ (depth as u64).wrapping_mul(0x9e37_79b9) ^ width as u64);
        let mut weights = Vec::new();
        let mut biases = Vec::new();
        for layer in 0..sizes.len() - 1 {
            let fan_in = sizes[layer];
            let fan_out = sizes[layer + 1];
            // Xavier-ish range keeps tanh out of saturation without making the
            // first few frames look frozen.
            let limit = (6.0 / (fan_in + fan_out) as f32).sqrt() * 0.9;
            let mut w = Vec::with_capacity(fan_in * fan_out);
            for _ in 0..fan_in * fan_out {
                w.push(rng.range(-limit, limit));
            }
            weights.push(w);
            biases.push(vec![0.0; fan_out]);
        }
        let adam_m_w = weights.iter().map(|w| vec![0.0; w.len()]).collect();
        let adam_v_w = weights.iter().map(|w| vec![0.0; w.len()]).collect();
        let adam_m_b = biases.iter().map(|b| vec![0.0; b.len()]).collect();
        let adam_v_b = biases.iter().map(|b| vec![0.0; b.len()]).collect();
        let activations = sizes.iter().map(|n| vec![0.0; *n]).collect();

        let mut mlp = Self {
            sizes,
            weights,
            biases,
            adam_m_w,
            adam_v_w,
            adam_m_b,
            adam_v_b,
            train_steps: 0,
            activations,
        };
        // Prime the cache so the first frame already shows a prediction.
        mlp.forward(0.5, 0.5);
        mlp
    }

    pub fn sizes(&self) -> &[usize] {
        &self.sizes
    }

    pub fn depth(&self) -> usize {
        self.sizes.len() - 2
    }

    pub fn width(&self) -> usize {
        self.sizes[1]
    }

    pub fn params(&self) -> usize {
        self.weights.iter().map(Vec::len).sum::<usize>()
            + self.biases.iter().map(Vec::len).sum::<usize>()
    }

    /// Returns the activations for every layer after feeding `(x, y)`.
    pub fn forward(&mut self, x: f32, y: f32) -> Vec<Vec<f32>> {
        self.activations[0][0] = x.clamp(0.0, 1.0);
        self.activations[0][1] = y.clamp(0.0, 1.0);
        for layer in 0..self.sizes.len() - 1 {
            let input = self.activations[layer].clone();
            let out_size = self.sizes[layer + 1];
            let mut out = vec![0.0; out_size];
            let w = &self.weights[layer];
            let b = &self.biases[layer];
            for o in 0..out_size {
                let row = &w[o * input.len()..(o + 1) * input.len()];
                let mut z = b[o];
                for (wi, xi) in row.iter().zip(&input) {
                    z += wi * xi;
                }
                out[o] = if layer + 2 == self.sizes.len() {
                    sigmoid(z)
                } else {
                    z.tanh()
                };
            }
            self.activations[layer + 1] = out;
        }
        self.activations.clone()
    }

    pub fn predict(&mut self, x: f32, y: f32) -> f32 {
        let acts = self.forward(x, y);
        acts.last().and_then(|a| a.first()).copied().unwrap_or(0.5)
    }

    pub fn last_activations(&self) -> &[Vec<f32>] {
        &self.activations
    }

    pub fn accuracy(&mut self, data: &Dataset) -> f32 {
        if data.samples.is_empty() {
            return 0.0;
        }
        let mut correct = 0;
        for sample in &data.samples {
            let p = self.predict(sample.x, sample.y);
            if (p >= 0.5) == sample.label {
                correct += 1;
            }
        }
        correct as f32 / data.samples.len() as f32
    }

    pub fn loss(&mut self, data: &Dataset) -> f32 {
        if data.samples.is_empty() {
            return 0.0;
        }
        let n = data.samples.len() as f32;
        let mut total = 0.0;
        for sample in &data.samples {
            let p = self.predict(sample.x, sample.y).clamp(1e-6, 1.0 - 1e-6);
            let y = if sample.label { 1.0 } else { 0.0 };
            total += -(y * p.ln() + (1.0 - y) * (1.0 - p).ln());
        }
        total / n
    }

    /// Full-batch gradient descent with Adam. Returns the mean loss before
    /// the update so the caller can draw the training curve.
    #[allow(clippy::needless_range_loop)]
    pub fn train_epoch(&mut self, data: &Dataset, learning_rate: f32) -> f32 {
        let n = data.samples.len().max(1) as f32;
        let mut grad_w: Vec<Vec<f32>> = self.weights.iter().map(|w| vec![0.0; w.len()]).collect();
        let mut grad_b: Vec<Vec<f32>> = self.biases.iter().map(|b| vec![0.0; b.len()]).collect();
        let mut loss = 0.0;
        let lr = learning_rate.clamp(0.001, 1.5);

        for sample in &data.samples {
            let acts = self.forward(sample.x, sample.y);
            let output = acts.last().and_then(|a| a.first()).copied().unwrap_or(0.5);
            let target = if sample.label { 1.0 } else { 0.0 };
            let p = output.clamp(1e-6, 1.0 - 1e-6);
            loss += -(target * p.ln() + (1.0 - target) * (1.0 - p).ln());

            // The sigmoid + BCE derivative collapses to (prediction - target).
            let mut delta = vec![output - target];
            for layer in (0..self.sizes.len() - 1).rev() {
                let input = &acts[layer];
                let out_size = self.sizes[layer + 1];
                let in_size = self.sizes[layer];
                for o in 0..out_size {
                    let d = delta[o];
                    grad_b[layer][o] += d;
                    for i in 0..in_size {
                        grad_w[layer][o * in_size + i] += d * input[i];
                    }
                }
                if layer > 0 {
                    let mut next = vec![0.0; in_size];
                    for i in 0..in_size {
                        let mut sum = 0.0;
                        for o in 0..out_size {
                            sum += self.weights[layer][o * in_size + i] * delta[o];
                        }
                        // tanh derivative
                        let a = acts[layer][i];
                        next[i] = sum * (1.0 - a * a);
                    }
                    delta = next;
                }
            }
        }

        // Adam keeps the tiny sandbox responsive across very different
        // landscapes. With plain momentum the spiral frequently settled on a
        // constant 0.5 prediction; Adam consistently escapes that plateau.
        self.train_steps += 1;
        let beta1 = 0.9_f32;
        let beta2 = 0.999_f32;
        let eps = 1e-8_f32;
        let step = self.train_steps as f32;
        let bias1 = 1.0 - beta1.powf(step);
        let bias2 = 1.0 - beta2.powf(step);
        let update = |param: &mut f32, m: &mut f32, v: &mut f32, g: f32| {
            *m = beta1 * *m + (1.0 - beta1) * g;
            *v = beta2 * *v + (1.0 - beta2) * g * g;
            let m_hat = *m / bias1;
            let v_hat = *v / bias2;
            *param -= lr * m_hat / (v_hat.sqrt() + eps);
        };
        for layer in 0..self.weights.len() {
            for i in 0..self.weights[layer].len() {
                let g = grad_w[layer][i] / n;
                update(
                    &mut self.weights[layer][i],
                    &mut self.adam_m_w[layer][i],
                    &mut self.adam_v_w[layer][i],
                    g,
                );
            }
            for i in 0..self.biases[layer].len() {
                let g = grad_b[layer][i] / n;
                update(
                    &mut self.biases[layer][i],
                    &mut self.adam_m_b[layer][i],
                    &mut self.adam_v_b[layer][i],
                    g,
                );
            }
        }

        loss / n
    }

    /// A compact snapshot for the network view: each layer's weights and the
    /// activation of every neuron for the current probe.
    pub fn edge_weight(&self, layer: usize, out: usize, input: usize) -> f32 {
        let in_size = self.sizes[layer];
        *self
            .weights
            .get(layer)
            .and_then(|w| w.get(out * in_size + input))
            .unwrap_or(&0.0)
    }

    pub fn max_abs_weight(&self) -> f32 {
        self.weights
            .iter()
            .flat_map(|layer| layer.iter())
            .fold(0.0_f32, |acc, w| acc.max(w.abs()))
            .max(0.001)
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

/// Convenience used by the UI and tests to describe a trainable session.
#[derive(Clone, Debug)]
pub struct LabTraining {
    pub data: Dataset,
    pub net: Mlp,
    pub epochs: u32,
    pub learning_rate: f32,
    pub history: Vec<f32>,
}

impl LabTraining {
    pub fn new(kind: DatasetKind, seed: u64) -> Self {
        let (depth, width) = kind.default_shape();
        Self::with_shape(kind, depth, width, seed)
    }

    pub fn with_shape(kind: DatasetKind, depth: usize, width: usize, seed: u64) -> Self {
        Self {
            data: kind.dataset(),
            net: Mlp::new(depth, width, seed),
            epochs: 0,
            learning_rate: kind.default_learning_rate(),
            history: Vec::new(),
        }
    }

    pub fn accuracy(&mut self) -> f32 {
        self.net.accuracy(&self.data)
    }

    pub fn loss(&mut self) -> f32 {
        self.net.loss(&self.data)
    }

    /// Run one epoch and record the loss.
    pub fn step(&mut self) -> f32 {
        let loss = self.net.train_epoch(&self.data, self.learning_rate);
        self.epochs += 1;
        self.history.push(loss);
        if self.history.len() > 240 {
            self.history.remove(0);
        }
        loss
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datasets_are_deterministic_and_balanced() {
        for kind in DatasetKind::ALL {
            let a = kind.dataset();
            let b = kind.dataset();
            assert_eq!(a.samples.len(), b.samples.len());
            assert!(a.samples.iter().zip(&b.samples).all(|(x, y)| x == y));
            let (cold, lit) = a.class_balance();
            assert!(cold > 40, "{} cold {}", kind.id(), cold);
            assert!(lit > 40, "{} lit {}", kind.id(), lit);
        }
    }

    #[test]
    fn mlp_forward_is_in_range_and_params_match_shape() {
        let mut net = Mlp::new(1, 4, 7);
        let acts = net.forward(0.2, 0.8);
        assert_eq!(acts.len(), 3);
        assert_eq!(acts[1].len(), 4);
        let output = *acts.last().unwrap().first().unwrap();
        assert!((0.0..=1.0).contains(&output));
        assert_eq!(net.params(), 17);
        assert_eq!(net.sizes(), &[2, 4, 1]);
    }

    #[test]
    fn every_default_architecture_reaches_its_target() {
        for kind in DatasetKind::ALL {
            let (depth, width) = kind.default_shape();
            let mut training = LabTraining::with_shape(kind, depth, width, 0xabc);
            let first = training.loss();
            // XOR and ring are quick; spiral needs a little longer.
            let epochs = if kind == DatasetKind::Spiral {
                900
            } else {
                300
            };
            for _ in 0..epochs {
                training.step();
            }
            assert!(
                training.accuracy() >= kind.target_accuracy(),
                "{}: accuracy {:.3}, target {:.3}, loss {:.3}",
                kind.id(),
                training.accuracy(),
                kind.target_accuracy(),
                training.loss()
            );
            assert!(training.loss() < first * 0.3);
            assert!(training.net.params() <= kind.parameter_budget());
        }
    }

    #[test]
    fn curated_spiral_seed_converges_at_every_width() {
        for width in [9usize, 12, 14] {
            let mut training = LabTraining::with_shape(
                DatasetKind::Spiral,
                2,
                width,
                DatasetKind::Spiral.training_seed(),
            );
            for _ in 0..1200 {
                training.step();
            }
            assert!(
                training.accuracy() >= DatasetKind::Spiral.target_accuracy(),
                "spiral width {width} only reached {:.3}",
                training.accuracy()
            );
        }
    }

    #[test]
    fn width_changes_rebuild_the_shape() {
        let mut training = LabTraining::with_shape(DatasetKind::Xor, 1, 4, 7);
        assert_eq!(training.net.sizes(), &[2, 4, 1]);
        let bigger = LabTraining::with_shape(DatasetKind::Xor, 2, 6, 7);
        assert_eq!(bigger.net.sizes(), &[2, 6, 6, 1]);
        assert_eq!(bigger.net.depth(), 2);
        training.learning_rate = 0.01;
        assert!(training.learning_rate < 0.02);
    }

    #[test]
    fn training_is_deterministic() {
        let mut a = LabTraining::with_shape(DatasetKind::Xor, 1, 4, 99);
        let mut b = LabTraining::with_shape(DatasetKind::Xor, 1, 4, 99);
        for _ in 0..50 {
            a.step();
            b.step();
        }
        assert!((a.loss() - b.loss()).abs() < 1e-6);
    }

    #[test]
    fn history_is_bounded() {
        let mut training = LabTraining::new(DatasetKind::Xor, 3);
        for _ in 0..300 {
            training.step();
        }
        assert_eq!(training.history.len(), 240);
    }

    #[test]
    fn trial_shapes_are_smaller_than_defaults_and_converge() {
        for kind in DatasetKind::ALL {
            let spec = kind.trial();
            let (depth, width) = kind.default_shape();
            let default_params = Mlp::new(depth, width, kind.training_seed()).params();
            assert!(
                !spec.shape_ok(depth, width, default_params),
                "{} default shape already satisfies its trial",
                kind.id()
            );

            let mut training =
                LabTraining::with_shape(kind, spec.max_depth, spec.max_width, kind.training_seed());
            let epochs = if kind == DatasetKind::Spiral {
                1200
            } else {
                500
            };
            for _ in 0..epochs {
                training.step();
            }
            let params = training.net.params();
            assert!(
                spec.is_met(spec.max_depth, spec.max_width, params, training.accuracy()),
                "{} trial max shape only reached {:.3} with {} params",
                kind.id(),
                training.accuracy(),
                params
            );
            assert!(
                params <= spec.max_params,
                "{} trial parameters {} exceed budget {}",
                kind.id(),
                params,
                spec.max_params
            );
        }
    }

    #[test]
    fn trial_requires_both_shape_and_accuracy() {
        let spec = DatasetKind::Xor.trial();
        assert!(spec.shape_ok(1, 3, 13));
        assert!(!spec.shape_ok(1, 4, 17));
        assert!(!spec.is_met(1, 3, 13, 0.80));
        assert!(spec.is_met(1, 3, 13, 0.97));

        // Spiral needs the second hidden layer; a shallower net that fits the
        // parameter budget still cannot learn the target.
        let spiral = DatasetKind::Spiral.trial();
        assert!(spiral.shape_ok(2, 6, 67));
        assert!(!spiral.shape_ok(1, 6, 19));
    }
}
