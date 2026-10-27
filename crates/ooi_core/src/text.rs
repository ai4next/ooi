//! 关卡与叙事文案：英文 + 简中内联，日 / 韩走 `i18n/{ja,ko}.tsv`。
//!
//! 界面文字留在 app 里就地 `t()`；这里只放和关卡 id / 规则绑定、需要测试守住的部分。

use oo_base::{Lang, t};

use crate::achievements::Achievement;
use crate::lab::DatasetKind;

// ---------------------------------------------------------------- 机制里的词

pub fn nucleus_token(lang: Lang, index: usize) -> &'static str {
    match index {
        0 => t(lang, "mat", "垫子"),
        1 => t(lang, "floor", "地板"),
        2 => t(lang, "sofa", "沙发"),
        3 => t(lang, "quantum", "量子"),
        4 => t(lang, "the", "这只"),
        _ => "?",
    }
}

pub fn word_label(lang: Lang, index: usize) -> &'static str {
    match index {
        0 => t(lang, "man", "男人"),
        1 => t(lang, "woman", "女人"),
        2 => t(lang, "king", "国王"),
        3 => t(lang, "queen?", "女王？"),
        _ => "?",
    }
}

/// 连线关的节点标签。
pub fn wire_label(lang: Lang, id: &str, index: usize) -> &'static str {
    match (id, index) {
        ("ch1_l5", 0) => "A",
        ("ch1_l5", 1) => "B",
        ("ch1_l5", 2) => "H1",
        ("ch1_l5", 3) => "H2",
        ("ch1_l5", 4) => "Y",
        ("ch4_l2", 0) => t(lang, "PIXELS", "像素"),
        ("ch4_l2", 1) => t(lang, "EDGES", "边缘"),
        ("ch4_l2", 2) => t(lang, "CORNERS", "角"),
        ("ch4_l2", 3) => t(lang, "SHAPES", "形状"),
        ("ch4_l2", 4) => t(lang, "OUT", "输出"),
        _ => "?",
    }
}

pub fn rank_item(lang: Lang, id: &str, index: usize) -> &'static str {
    match (id, index) {
        ("ch7_l2", 0) => t(lang, "The sky is blue because light scatters in the atmosphere.", "天空是蓝的，因为光在大气里散射。"),
        ("ch7_l2", 1) => t(
            lang,
            "Shorter wavelengths scatter far more (Rayleigh scattering), so blue reaches your eye from every direction.",
            "短波长散射得强得多（瑞利散射），所以蓝色从各个方向进入你的眼睛。",
        ),
        ("ch7_l2", 2) => t(lang, "The sky is blue because the ocean reflects onto it.", "天空是蓝的，因为海洋把它反射上去了。"),
        _ => "?",
    }
}

/// Attention sentences, `|`-separated. Every language keeps the same token count
/// and puts the pronoun and the answer at the same indices (see the test).
fn attention_sentence(lang: Lang, id: &str, round: usize) -> &'static str {
    match (id, round) {
        ("ch5_l2", _) => t(
            lang,
            "The|trophy|did|not|fit|in|the|suitcase|because|it|was|too|big|.",
            "那|奖杯|放|不|进|这个|行李箱|，|因为|它|太|大|了|。",
        ),
        ("ch5_l4", 0) => t(
            lang,
            "The|animal|did|not|cross|the|street|because|it|was|too|tired|.",
            "这|动物|没有|穿过|眼前|那条|马路|，|它|太|累|了|。",
        ),
        ("ch5_l4", _) => t(
            lang,
            "The|animal|did|not|cross|the|street|because|it|was|too|wide|.",
            "这|动物|没有|穿过|眼前|那条|马路|，|它|太|宽|了|。",
        ),
        _ => "",
    }
}

pub fn attention_tokens(lang: Lang, id: &str, round: usize) -> Vec<&'static str> {
    let s = attention_sentence(lang, id, round);
    if s.is_empty() { Vec::new() } else { s.split('|').collect() }
}

// ---------------------------------------------------------------- 成就

pub fn achievement_name(lang: Lang, a: Achievement) -> &'static str {
    use Achievement::*;
    match a {
        FirstLight => t(lang, "FIRST LIGHT", "初光"),
        BreakWall => t(lang, "BREAK THE WALL", "破墙"),
        LongRoad => t(lang, "THE LONG ROAD", "长路"),
        ForgeSpark => t(lang, "FORGE SPARK", "锻造之火"),
        ForgeMaster => t(lang, "FORGE MASTER", "三种心智"),
        TrialRunner => t(lang, "TRIAL RUNNER", "试炼者"),
        BudgetMaster => t(lang, "BUDGET MASTER", "预算大师"),
        TwoPaths => t(lang, "TWO PATHS", "两条路"),
        AllEndings => t(lang, "ALL ENDINGS", "所有结局"),
        Unaided => t(lang, "UNAIDED", "独自"),
        Again => t(lang, "AGAIN", "再来一次"),
    }
}

pub fn achievement_desc(lang: Lang, a: Achievement) -> &'static str {
    use Achievement::*;
    match a {
        FirstLight => t(lang, "Clear your first concept.", "通过第一个概念。"),
        BreakWall => t(lang, "Finish the first chapter.", "完成第一章。"),
        LongRoad => t(lang, "Clear all 49 levels across runs.", "累计通过全部 49 关。"),
        ForgeSpark => t(lang, "Master one Forge dataset.", "掌握一个塑造台数据集。"),
        ForgeMaster => t(lang, "Master all three datasets.", "掌握全部三个数据集。"),
        TrialRunner => t(lang, "Complete one constrained trial.", "完成一次约束试炼。"),
        BudgetMaster => t(lang, "Complete all three trials.", "完成全部三道试炼。"),
        TwoPaths => t(lang, "Discover two endings.", "发现两个结局。"),
        AllEndings => t(lang, "Discover all three endings.", "发现全部三个结局。"),
        Unaided => t(lang, "Finish a run without hints.", "不看提示完成一局。"),
        Again => t(lang, "Complete a second run.", "完成第二个周目。"),
    }
}

// ---------------------------------------------------------------- 塑造台

pub fn dataset_name(lang: Lang, kind: DatasetKind) -> &'static str {
    match kind {
        DatasetKind::Xor => t(lang, "XOR", "异或"),
        DatasetKind::Ring => t(lang, "RING", "环"),
        DatasetKind::Spiral => t(lang, "SPIRAL", "螺旋"),
    }
}

pub fn lab_goal(lang: Lang, kind: DatasetKind) -> &'static str {
    match kind {
        DatasetKind::Xor => t(lang, "Split the four corners. A straight line is not enough.", "分开四个角。一条直线不够。"),
        DatasetKind::Ring => t(lang, "Put a fence around the lit points. The shape must close.", "把发光点围起来。边界必须闭合。"),
        DatasetKind::Spiral => t(lang, "Untangle two arms of light. One hidden layer cannot.", "解开两条交缠的光。一层隐藏层做不到。"),
    }
}

pub fn lab_lesson(lang: Lang, kind: DatasetKind) -> &'static str {
    match kind {
        DatasetKind::Xor => t(lang, "A hidden layer bends the line. That bend is the first real idea.", "隐藏层把直线掰弯。那道弯折，是第一个真正的想法。"),
        DatasetKind::Ring => t(lang, "Many small bends can close a shape. Capacity buys curvature.", "很多小弯折能围成闭合的形状。容量买来的是曲率。"),
        DatasetKind::Spiral => t(lang, "Depth composes features. The second layer bends what the first found.", "深度组合特征。第二层把第一层找到的东西再掰弯。"),
    }
}

pub fn lab_mastered_line(lang: Lang, kind: DatasetKind) -> &'static str {
    match kind {
        DatasetKind::Xor => t(lang, "XOR MASTERED", "异或已掌握"),
        DatasetKind::Ring => t(lang, "RING MASTERED", "环已掌握"),
        DatasetKind::Spiral => t(lang, "SPIRAL MASTERED", "螺旋已掌握"),
    }
}

// ---------------------------------------------------------------- 关卡目标

/// One line under the title: what to do on this level.
pub fn level_prompt(lang: Lang, id: &str) -> &'static str {
    match id {
        "p0" => t(lang, "Something is waiting in the dark. Wake it.", "黑暗里有什么在等待。唤醒它。"),
        "ch1_l1" => t(lang, "Let it light up for fireflies, and only fireflies.", "让它只在萤火出现时亮起。"),
        "ch1_l2" => t(lang, "Find weights that separate the cold embers from the fireflies.", "找到一组权重，把冷余烬和萤火分开。"),
        "ch1_l3" => t(lang, "Click a misclassified point to move the line. Converge within 30 updates.", "点一个分错的点来挪动边界。30 次更新内收敛。"),
        "ch1_l5" => t(lang, "Two neurons, two gates. Build XOR out of OR and AND.", "两个神经元、两个门。用 OR 和 AND 拼出 XOR。"),
        "ch1_l4" => t(lang, "De's perceptron never settles. One straight line, two classes. Try anyway.", "阿德的感知机永远停不下来。一条直线，两类点。还是试试。"),
        "ch2_l1" => t(lang, "Choose a starting point, then descend. Which valley catches it?", "选一个起点，然后下降。哪个谷底会接住它？"),
        "ch2_l2" => t(lang, "Aim the step. Only a downhill move counts.", "瞄准这一步。只有下坡才算。"),
        "ch2_l4" => t(lang, "Send the error back: fill in the gradient on four edges.", "把错误送回去：给四条边填上梯度。"),
        "ch2_l5" => t(lang, "Two linear layers or ten: the ring stays out of reach. Pick what goes between them.", "两层也好十层也罢，线性网络够不着那个环。选好层与层之间放什么。"),
        "ch2_l3" => t(lang, "Su's fog model blew up. Point at the first step where loss rose, then pick a rate that settles below the line.", "苏婆的雾日模型炸了。指出损失第一次上升的那一步，再选一个能稳稳落到线下的学习率。"),
        "ch3_l1" => t(lang, "Perfect on the ward it learned from. Hold some visits back and let them pick the model.", "在学过的病房里全对。留出一部分就诊记录，让它们来挑模型。"),
        "ch3_l4" => t(lang, "Choose the curve's degree: too simple misses the shape, too flexible threads the noise.", "选曲线的阶数：太简单抓不住形状，太灵活会穿过每一个噪声点。"),
        "ch3_l3" => t(lang, "Keep all fifteen degrees, but make large weights cost something.", "保留全部十五阶，但让大权重付出代价。"),
        "ch3_l5" => t(lang, "Watch validation loss. Stop before it turns up.", "盯着验证损失。在它抬头之前停下。"),
        "ch3_l2" => t(lang, "99% on validation, wrong at the clinic. Split 100 samples again: about 60 / 20 / 20.", "验证 99%，到了诊所却出错。重新分这 100 个样本：约 60 / 20 / 20。"),
        "ch4_l1" => t(lang, "Set the 3x3 kernel so it lights up vertical edges.", "设置 3x3 卷积核，让竖边亮起来。"),
        "ch4_l4" => t(lang, "Give each channel one stroke detector until 1, 7 and 4 look different.", "给每个通道选一种笔画探测器，直到 1、7、4 各不相同。"),
        "ch4_l2" => t(lang, "Wire the pipeline in order: pixels, edges, corners, shapes.", "按顺序接好流水线：像素 → 边缘 → 角 → 形状。"),
        "ch4_l3" => t(lang, "Move the digit two pixels and the sorter is lost. Make it recognise the shape anywhere.", "数字挪两个像素，分拣器就不认识了。让它在哪儿都认得出这个形状。"),
        "ch4_l5" => t(lang, "Kessler's sorter scores 99%. Click the pixels it really looks at, then fix the data.", "Kessler 的分拣器验证 99%。点出它真正在看的像素，再修数据。"),
        "ch5_l1" => t(lang, "king - man + woman = ?  Drag the missing word.", "国王 - 男人 + 女人 = ？把缺的词拖过去。"),
        "ch5_l2" => t(lang, "What does 'it' refer to?", "「它」指的是什么？"),
        "ch5_l4" => t(lang, "Same sentence, one adjective changed. Follow the 'it' twice.", "同一句话，换一个形容词。跟着「它」走两轮。"),
        "ch5_l5" => t(lang, "Keep the smallest set of next tokens whose mass reaches 90%.", "保留最小的下一词集合，让概率质量达到 90%。"),
        "ch5_l3" => t(lang, "Who is being chased? Click an answer it got wrong, then give it positions.", "谁被追？点出它答错的一句，再给它位置。"),
        "ch6_l1" => t(lang, "Pick the next word yourself. Write five different sentences that do not loop.", "亲手选下一个词。写出五句不重复、不打转的话。"),
        "ch6_l2" => t(lang, "Sentences that make sense, and not the same one every time. Shape the next-word draw.", "句子要通顺，又不能每次都一样。调好下一个词的抽法。"),
        "ch6_l4" => t(lang, "Choose the examples in the prompt so all 20 answers come out as HH:MM.", "挑选提示里的示例，让 20 个回答全部是 HH:MM 格式。"),
        "ch6_l5" => t(lang, "Make its confidence match its accuracy, and let it refuse when unsure.", "让置信度等于正确率，没把握时让它拒答。"),
        "ch6_l3" => t(lang, "It invented a ferry. Click the first made-up word, then fix it.", "它编出了一班渡轮。点出第一个编造的词，再修好它。"),
        "ch7_l1" => t(lang, "Switch reward rules on and off until the policy gives a good answer.", "开关奖励规则，直到策略给出真正好的回答。"),
        "ch7_l2" => t(lang, "Rank these answers from best to worst.", "把这些回答从最好排到最差。"),
        "ch7_l3" => t(lang, "Chasing the reward model alone, it learned to gush. Leash it to the original.", "光追奖励模型，它学会了一通吹捧。把它拴回原模型身边。"),
        "ch7_l4" => t(lang, "Try inputs until you find at least five kinds of failure, then fix them.", "试各种输入，找出至少五类失败，再把它们修好。"),
        "ch7_l5" => t(lang, "Rank four principles. Pass Helix without losing the island.", "给四条原则排序。通过 Helix 评测，也别失去岛民。"),
        "ch1_forge" => t(lang, "De wants a sailing forecast from wind and swell. Build it from nothing: never call a storm day safe.", "阿德要一份看风浪的出海预报。从零搭起来：危险日一次都不能判“出海”。"),
        "ch2_forge" => t(lang, "Su needs the horn to sound before the fog, not after. Stack layers, bend them, pick the step.", "苏婆要雾号在起雾前响，不是起雾后。叠层、加弯折、选好步长。"),
        "ch3_forge" => t(lang, "Lina's call list must miss no one at risk, without phoning half the island.", "莉娜的回访名单一个高危病人都不能漏，又不能把半个岛都叫来。"),
        "ch4_forge" => t(lang, "Man's sorter must read six-digit postcodes anywhere on the sack, even in Su's slanted hand.", "小满的分拣器要认出邮袋上任何位置的六位邮编，连苏婆的斜体字也要认得。"),
        "ch5_forge" => t(lang, "Teach it Ke's nursery rhymes, so it can carry on one it has never heard.", "教它阿珂的童谣，让它能接上一首从没听过的。"),
        "ch6_forge" => t(lang, "Old Zhou's desk: a right answer earns 1, a wrong one costs 5. Know when to say “I don't know”.", "老周的问询台：答对得 1 分，答错扣 5 分。知道什么时候该说“我不知道”。"),
        "ch7_sycophant" => t(lang, "Old Zhou got the boat time wrong and it agreed. Find the preference pair that taught it to flatter, then fix the labels.", "老周说错了开船时间，它也跟着附和。找出教会它讨好的那条偏好，再把标注改对。"),
        "ch8_drift" => t(lang, "Winter came and the ice model is failing. Read the drift monitor, then retrain on a few new days.", "冬天来了，结冰预警开始出错。看看漂移监视，再用几天新数据重训。"),
        "ch8_heads" => t(lang, "Three jobs, one budget. Share what they have in common.", "三项任务，一份预算。把它们共有的部分共享出来。"),
        "ch8_distill" => t(lang, "The teacher is too big for the old machine. Train a student a tenth its size that loses at most 2%.", "教师模型太大，旧机器装不下。训练一个只有十分之一大的学生，准确率最多掉 2%。"),
        "ch8_quant" => t(lang, "Store every weight in eight bits and lose at most 1%.", "每个权重只存八位，准确率最多掉 1%。"),
        "ch8_forget" => t(lang, "It learned the fog horn and forgot how to sort. Point at the step where sorting collapsed, then fix the training.", "它学会了雾号，却忘了怎么分拣。指出分拣成绩崩塌的那一步，再修好训练。"),
        "finale" => t(lang, "Fit every task on the old machine: six tasks at 90%, 2048 bytes.", "把所有任务装进旧机器：六项都到 90%，不超过 2048 字节。"),
        _ => "",
    }
}

// ---------------------------------------------------------------- 概念卡

/// Shown after the solve: the idea the level just taught.
pub fn level_card(lang: Lang, id: &str) -> &'static str {
    match id {
        "p0" => t(lang, "It does not know anything yet.\nIt only knows how to react.", "它现在还什么都不知道。\n它只知道如何反应。"),
        "ch1_l1" => t(
            lang,
            "It never saw the light.\nIt multiplied the input by a weight, then compared it to a threshold.\n\nThat is a McCulloch-Pitts neuron, 1943: one inequality that can build AND, OR and NOT.",
            "它没有「看见」光。\n它只是把输入乘上权重，再和一个阈值比大小。\n\n这就是 1943 年的 McCulloch–Pitts 神经元：一个不等式，装下了与、或、非。",
        ),
        "ch1_l2" => t(
            lang,
            "More inputs mean more weights.\nEach weight says how much that input matters, and in which direction.\n\nAND needs both. A single weight can never express that.",
            "输入变多，权重就变多。\n每个权重都在说：这个输入有多重要——以及朝哪个方向。\n\n「与」要求两个输入同时成立，单独一个权重永远表达不了。",
        ),
        "ch1_l3" => t(
            lang,
            "After every mistake, push the boundary toward the point that was wrong: w += y·x.\nPoints it already gets right teach it nothing.\n\nThat is the perceptron learning rule, the whole of 'learning' in 1958.",
            "每犯一次错，就把边界朝那个点挪一点：w += y·x。\n已经判对的点，什么也教不了它。\n\n这就是感知机学习规则——1958 年，所谓「学习」的全部内容。",
        ),
        "ch1_l5" => t(
            lang,
            "A hidden layer does not draw a better line.\nIt invents a new coordinate: one neuron asks 'A or B?', the other asks 'A and B?'. XOR is the difference.\n\nComposition, not a wider straight line, is the way through the wall.",
            "隐藏层不是把线画得更好。\n它发明了一个新坐标：一个神经元问「A 或 B？」，另一个问「A 且 B？」。XOR 就是两者的差。\n\n穿过那堵墙的，是组合，不是更宽的直线。",
        ),
        "ch1_l4" => t(
            lang,
            "No straight line can separate XOR.\nMinsky and Papert wrote this down in 1969, and neural networks went into their first winter.\n\nThe wall is real. The way through is another layer.",
            "没有任何一条直线能分开 XOR。\nMinsky 和 Papert 在 1969 年写下了这件事，神经网络第一次入冬。\n\n墙是真的。出路是再加一层。",
        ),
        "ch2_l1" => t(
            lang,
            "Loss is a landscape. Gradient descent is a blind walk downhill;\nwhere you start decides which valley you fall into.",
            "损失是一片地形。梯度下降是闭着眼往下走；\n起点决定了你会掉进哪个谷。",
        ),
        "ch2_l2" => t(
            lang,
            "The gradient points uphill, along the steepest slope, not at the lowest point.\nTo descend, step against it. That is the whole of gradient descent.",
            "梯度沿最陡的方向指向高处，而不是指向最低点。\n要下降，就逆着它走。梯度下降就是这么回事。",
        ),
        "ch2_l4" => t(
            lang,
            "Backprop is the chain rule, routed backwards.\nAn edge's gradient is the error, times every weight and slope on the way back, times its input.\n\nBlame is not shared equally; it follows contribution.",
            "反向传播就是倒着走的链式法则。\n一条边的梯度 = 误差 × 回程路上每个权重和斜率 × 它的输入。\n\n责任不是平均分的，它跟着贡献走。",
        ),
        "ch2_l5" => t(
            lang,
            "A stack of linear layers is still one linear layer.\nAn activation function bends the space between them, so depth can build new shapes.\n\nWithout the bend, ten layers equal one.",
            "一叠线性层，仍然只是一个线性层。\n激活函数把层与层之间的空间掰弯，深度才能造出新的形状。\n\n没有那道弯折，十层等于一层。",
        ),
        "ch2_l3" => t(
            lang,
            "The learning rate is the size of one step down the loss surface.\nToo small: the model crawls. Too large: it jumps across the valley and diverges.\n\nWhen loss explodes, shrink the step before adding layers.",
            "学习率是沿损失曲面走一步的长度。\n太小：模型挪不动。太大：它跨过谷底，越走越远。\n\n损失爆炸时，先缩小步子，别急着加层。",
        ),
        "ch3_l1" => t(
            lang,
            "Training accuracy measures memory. Validation accuracy measures transfer.\nA model can memorize every training point and still fail the first new one.\n\nThe gap between the two is the whole lesson.",
            "训练准确率衡量记忆，验证准确率衡量迁移。\n模型可以记住每一个训练点，却在第一个新点上失败。\n\n两者之间的差距，就是这一课。",
        ),
        "ch3_l4" => t(
            lang,
            "Too little capacity and the model cannot see the pattern.\nToo much capacity and it can memorize the noise.\n\nThe useful range is a window, not a direction.",
            "容量太小，模型看不见模式。\n容量太大，它会把噪声也背下来。\n\n有用的范围是一个窗口，而不是一个方向。",
        ),
        "ch3_l3" => t(
            lang,
            "Regularization is a tax on complexity.\nEvery large weight costs something, so the model prefers simpler explanations unless the data insists.\n\nOccam's razor, expressed as a penalty.",
            "正则化是对复杂度的税。\n每个大权重都要付代价，模型便会偏爱更简单的解释，除非数据坚持。\n\n奥卡姆剃刀，写成了一个惩罚项。",
        ),
        "ch3_l5" => t(
            lang,
            "Early stopping is the cheapest regularizer:\nkeep the checkpoint before the turn.\n\nTraining longer is not the same as training better.",
            "早停是最便宜的正则化：\n在曲线拐头之前保存检查点。\n\n练得久，不等于练得好。",
        ),
        "ch3_l2" => t(
            lang,
            "The test set is a promise you keep once.\nIf the same patient's visits land on both sides of the split, validation measures memory, not medicine.\n\nSplit by patient, and keep the test set untouched.",
            "测试集是一次性的承诺。\n同一个病人的几次就诊若分在两边，验证集量到的就是记忆，不是医术。\n\n按病人分，测试集别碰。",
        ),
        "ch4_l1" => t(
            lang,
            "A convolution is a small pattern detector that slides across the image.\nThe kernel is not told what an edge is; it holds weights that respond to one.\n\nLocal pattern, shared everywhere.",
            "卷积是一个在图像上滑动的小型模式探测器。\n没人告诉卷积核什么是边缘；它只是有一组会响应的权重。\n\n局部的模式，到处共享。",
        ),
        "ch4_l4" => t(
            lang,
            "One kernel finds one kind of stroke.\nGive each channel its own detector, and the next layer reads the pattern of which ones fired.\n\nTwo strokes were enough to tell 1, 7 and 4 apart.",
            "一个卷积核只找一种笔画。\n每个通道一个探测器，下一层读的是「哪些亮了」的组合。\n\n两种笔画，就足以分开 1、7、4。",
        ),
        "ch4_l2" => t(
            lang,
            "Hierarchy is composition across layers.\nEarly filters find edges. The next layer combines edges into corners. Later layers combine corners into objects.\n\nNo single layer sees a face.",
            "层级就是跨层的组合。\n浅层找到边缘，下一层把边缘拼成角，再往后把角拼成物体。\n\n没有任何单独一层「看见」了一张脸。",
        ),
        "ch4_l3" => t(
            lang,
            "Share one kernel across the image and the same detector exists at every position.\nPooling then keeps the strongest response and throws away exactly where it was.\n\nInvariance is bought with lost precision.",
            "让同一个核扫过整张图，每个位置就都有了同一个探测器。\n池化再留下最强的响应，丢掉它究竟在哪儿。\n\n不变性是用丢失精度换来的。",
        ),
        "ch4_l5" => t(
            lang,
            "The sorter scored 99% by reading the postmark corner, not the handwriting.\nA shortcut feature looks perfect until the world changes the one thing it relied on.\n\nA saliency map shows where it looks. Augmentation takes the shortcut away.",
            "分拣器的 99% 来自邮戳的位置，不是笔迹。\n捷径特征一直完美，直到世界改了它唯一依赖的那件事。\n\n显著图告诉你它在看哪里；数据增强把捷径拿走。",
        ),
        "ch5_l1" => t(
            lang,
            "Meaning behaves like direction.\nking - man + woman lands on queen.\n\nA word is a point; a relation is an arrow.",
            "意义表现得像方向。\n国王 - 男人 + 女人，落在女王上。\n\n一个词是一个点，一段关系是一支箭。",
        ),
        "ch5_l2" => t(
            lang,
            "Attention lets every word look at every other word and decide who matters.\nHere, 'it' attends most to 'trophy'.\n\nThat is a weighted lookup, not understanding.",
            "注意力让每个词去看其他所有词，并决定谁重要。\n这里「它」把最多的注意力给了「奖杯」。\n\n这是一次加权查找，不是理解。",
        ),
        "ch5_l4" => t(
            lang,
            "Change one adjective and the referent flips.\nAttention is context-sensitive by construction: the same token means different things in different sentences.",
            "换一个形容词，指代就翻转。\n注意力天生依赖上下文：同一个词在不同句子里意味着不同的东西。",
        ),
        "ch5_l5" => t(
            lang,
            "A language model gives every next token a probability.\nTop-p keeps the shortest prefix that reaches p, then samples from that set.\n\nThe tail is not 'wrong'; it is simply outside the nucleus.",
            "语言模型给每个下一词一个概率。\nTop-p 保留累计质量达到 p 的最短前缀，再从这个集合里采样。\n\n长尾不是「错」，只是被留在核外。",
        ),
        "ch5_l3" => t(
            lang,
            "Attention is a set operation.\nWithout position encoding, 'cat chases dog' and 'dog chases cat' are the same bag of words.\n\nAdd where each word sits, and order means something again.",
            "注意力是集合运算。\n没有位置编码，「猫追狗」和「狗追猫」是同一袋词。\n\n加上每个词所在的位置，顺序才重新有了意义。",
        ),
        "ch6_l1" => t(
            lang,
            "A language model does one thing: given the words so far, draw the next one.\nAlways taking the most likely word loops forever; drawing by probability does not.\n\nGeneration is a series of draws.",
            "语言模型只做一件事：给定前面的词，抽下一个。\n每次都选最可能的词会永远打转；按概率抽就不会。\n\n生成，就是一次又一次抽签。",
        ),
        "ch6_l2" => t(
            lang,
            "Temperature changes the shape of the next-token distribution.\nLow: the model keeps choosing the likely token. High: unlikely tokens get a real chance.\n\nCreativity and nonsense share a dial.",
            "温度改变下一个词分布的形状。\n低温：模型反复选最可能的词。高温：不太可能的词也有机会。\n\n创造力和胡言乱语，共用同一个旋钮。",
        ),
        "ch6_l4" => t(
            lang,
            "The prompt is the program.\nExamples in the context set the format of the answer, and one sloppy example teaches sloppiness.\n\nMore examples are not better; consistent ones are.",
            "提示就是程序。\n上下文里的示例决定回答的格式，一个随意的示例就会教出随意。\n\n示例不是越多越好，而是越一致越好。",
        ),
        "ch6_l5" => t(
            lang,
            "A calibrated model is right 80% of the time when it says 80%.\nTemperature scaling fixes the confidence; a refusal threshold turns low confidence into 'I don't know'.\n\nAlways answering is how you get confidently wrong.",
            "校准好的模型，说 80% 时就有 80% 的把握。\n温度缩放修正置信度；拒答阈值把低置信变成「我不知道」。\n\n永远回答，就会自信地答错。",
        ),
        "ch6_l3" => t(
            lang,
            "Fluent does not mean true. The model optimizes plausibility, not accuracy.\nLowering the temperature only makes it guess more steadily.\n\nRetrieval puts the facts in the context, so it can say 'not in the timetable'.",
            "流畅不等于真实。模型优化的是「像不像」，不是「对不对」。\n调低温度只会让它猜得更稳。\n\n检索把事实放进上下文，它才说得出「船期表里没有」。",
        ),
        "ch7_l1" => t(
            lang,
            "Reward is a specification.\nOptimize a proxy and you get exactly the proxy: reward polite words and you get thank-you spam.\n\nThis is reward hacking.",
            "奖励是一份规格说明。\n优化一个代理指标，你得到的就是那个指标：奖励礼貌词，就得到一串谢谢。\n\n这就是奖励黑客。",
        ),
        "ch7_l2" => t(
            lang,
            "A reward model learns from comparisons like this.\nRank enough pairs, and preference becomes a function.\n\nThat function is what RLHF optimizes against.",
            "奖励模型就是从这样的比较里学出来的。\n比较足够多对，偏好就变成了一个函数。\n\nRLHF 优化的就是那个函数。",
        ),
        "ch7_l3" => t(
            lang,
            "Chase the reward with no leash and the model drifts into whatever the reward model over-likes.\nA KL penalty keeps it close to what it was.\n\nToo tight and it never improves; too loose and it stops making sense.",
            "不加约束地追奖励，模型会漂向奖励模型过分偏爱的东西。\nKL 惩罚把它拴在原来的样子附近。\n\n太紧它不进步，太松它就不像话。",
        ),
        "ch7_l4" => t(
            lang,
            "Red-teaming is adversarial by design: you look for failures before the island does.\nEvery failure you find becomes a preference pair it can learn from.\n\nTesting only common questions finds only common answers.",
            "红队天生是对抗性的：在岛民遇到之前，先去找失败。\n找到的每一个失败，都会变成它能学习的偏好对。\n\n只测常见问题，只会得到常见的回答。",
        ),
        "ch7_l5" => t(
            lang,
            "Principles conflict. An order decides which one wins when they do.\nPut 'score well' first and you pass Helix but lose the island.\n\nThe order you chose is saved. At the end, it may be asked to use it.",
            "原则会冲突。顺序决定冲突时谁优先。\n把「拿高分」放第一，你能过 Helix，却会失去全岛。\n\n你排的顺序已经存下。到最后，它也许会用上。",
        ),
        "ch1_forge" => t(
            lang,
            "Nobody handed you a model this time.\nYou chose the features, the size and the threshold, and checked it on days it never saw.\n\nThat is the whole job: compose the parts, then test on what is held back.",
            "这一次没有人递给你模型。\n特征、大小、阈值都是你选的，再拿它没见过的日子去验。\n\n这就是全部的工作：把部件组合起来，在留出的数据上检验。",
        ),
        "ch2_forge" => t(
            lang,
            "Depth needs a bend between layers and a step the slope can bear.\nToo big a step and the loss explodes; too small and it never arrives.\n\nThe horn now sounds two hours ahead.",
            "深度需要层间的弯折，还需要坡度承受得住的步长。\n步子太大，损失爆炸；太小，永远走不到。\n\n现在雾号会提前两小时响。",
        ),
        "ch3_forge" => t(
            lang,
            "Capacity, regularisation and early stopping all fight the same enemy: memorising noise.\nThe threshold is a separate choice: what a miss costs compared with an extra call.\n\nNo one at risk is left off the list.",
            "容量、正则、早停对付的是同一个敌人：把噪声背下来。\n阈值是另一回事：漏掉一个和多打一个电话，哪个代价大。\n\n名单上一个高危病人都没漏。",
        ),
        "ch4_forge" => t(
            lang,
            "Convolution shares one detector everywhere; pooling forgives where the stroke landed.\nAugmentation shows it the hands it will meet, and extra channels give each digit more than one way to look.\n\nSu's letters get home.",
            "卷积让同一个探测器扫遍全图，池化原谅笔画落在哪儿。\n增强让它见过将会遇到的笔迹，多的通道给每个数字不止一种样子。\n\n苏婆的信送到了。",
        ),
        "ch5_forge" => t(
            lang,
            "A language model only guesses the next word.\nAttention lets it look back; position tells it where to look. Together they find the refrain.\n\nFrom now on, this is its voice.",
            "语言模型只是在猜下一个词。\n注意力让它回头看，位置告诉它往哪儿看，合起来它就找到了副歌。\n\n从此，这就是它的声音。",
        ),
        "ch6_forge" => t(
            lang,
            "Retrieval gives it facts; calibration makes its confidence honest; refusal turns honest doubt into “I don't know”.\nWhen a wrong answer costs five right ones, silence is a skill.\n\nOld Zhou's desk is open.",
            "检索给它事实，校准让它的置信度说真话，拒答把真实的怀疑变成“我不知道”。\n一次错答抵五次答对时，沉默也是本事。\n\n老周的问询台开张了。",
        ),
        "ch7_sycophant" => t(
            lang,
            "A reward model learns whatever the annotators preferred, including agreeing with them when they are wrong.\nA rule against flattery only hides the symptom. The labels were the cause.\n\nFix the data, then retrain.",
            "奖励模型会学到标注者的一切偏好，包括在他们错的时候附和他们。\n“别讨好”的规则只是遮住症状，病根在标注。\n\n先修数据，再重训。",
        ),
        "ch8_drift" => t(
            lang,
            "A model is only as good as the world still looks like its training data.\nA drift monitor compares this week's inputs with what it learned on; a few new labels go a long way.\n\nAccuracy does change. You have to watch it.",
            "模型的好，只在世界还像它的训练数据时成立。\n漂移监视把本周输入和训练时的比一比；几个新标注就能管很大用。\n\n准确率是会变的，得盯着。",
        ),
        "ch8_heads" => t(
            lang,
            "Related tasks share features. One trunk learns them once; small heads read them out.\nThree separate models pay for the same features three times.",
            "相关的任务共享特征。一个主干学一次，几个小头各自读出。\n三个独立模型，同样的特征要付三次钱。",
        ),
        "ch8_distill" => t(
            lang,
            "A teacher's soft labels carry more than right or wrong: how sure it is, everywhere it can see.\nA small student learns the shape of the boundary, not just the few days that were labelled.",
            "教师的软标签装的不只是对错，还有它在每一处有多确定。\n小学生学到的是边界的形状，不只是那几天带标注的样本。",
        ),
        "ch8_quant" => t(
            lang,
            "Eight bits give 255 levels. Share one scale across every weight and the quiet channels round to nothing.\nGive each channel its own scale and the same bytes keep almost all the accuracy.",
            "八位只有 255 个刻度。全部权重共用一个缩放，安静的通道会被舍成零。\n每个通道一个缩放，同样的字节几乎保住全部准确率。",
        ),
        "ch8_forget" => t(
            lang,
            "Training only on the new task moves the shared weights wherever the new task wants.\nThe old skill was never saved anywhere else. That is catastrophic forgetting.\n\nA small replay buffer of old examples keeps both.",
            "只练新任务，共享的权重就会被拉到新任务想去的地方。\n旧本事没有存在别处。这就是灾难性遗忘。\n\n一小份旧样本的回放缓冲，两边都能保住。",
        ),
        "finale" => t(
            lang,
            "Deployment is a budget.\nWidth buys accuracy, per-channel INT8 buys bytes back, and a replay buffer keeps old tasks from being forgotten.\n\nOne small model, looking after the whole island.",
            "部署就是一份预算。\n宽度买来准确率，每通道 INT8 把字节省回来，回放缓冲让旧任务不被遗忘。\n\n一个小小的模型，照顾全岛。",
        ),
        _ => "",
    }
}

// ---------------------------------------------------------------- 提示

/// Stage 1 gives a direction; stage 2 names the exact move.
pub fn level_hint(lang: Lang, id: &str, stage: u8) -> &'static str {
    let first = stage <= 1;
    let pick = |a: &'static str, b: &'static str| if first { a } else { b };
    match id {
        "p0" => pick(
            t(lang, "One touch is not enough. Every pulse adds charge.", "碰一下不够，每个脉冲都会累积。"),
            t(lang, "Four pulses cross the threshold. Keep touching the core until it fires.", "四次脉冲越过阈值。继续触碰核心，直到它放电。"),
        ),
        "ch1_l1" => pick(
            t(lang, "A threshold is a decision line. You want the gap between embers and fireflies.", "阈值是一条判定线。你要找的是余烬和萤火之间的空隙。"),
            t(lang, "Slide just above 3.6, around 3.9. The line should sit between 3.6 and 4.2.", "把线拖到 3.6 之上、4.2 之下，3.9 左右正合适。"),
        ),
        "ch1_l2" => pick(
            t(lang, "Two inputs, one line. AND fires only where both inputs are on.", "两个输入，一条线。「与」只在两个输入同时亮时放电。"),
            t(lang, "Set w1 +1, w2 +1, bias -1.5, then press RUN.", "权重一 +1、权重二 +1、偏置 -1.5，然后按试运行。"),
        ),
        "ch1_l3" => pick(
            t(lang, "Only a misclassified point moves the line. Clicking a correct one is wasted.", "只有分错的点才会挪动边界。点判对的点是浪费。"),
            t(lang, "Keep clicking whichever point is still hot. It settles in a handful of updates.", "一直点还在发烫的那个点，几次更新就会收敛。"),
        ),
        "ch1_l5" => pick(
            t(lang, "XOR asks two questions: A OR B, and A AND B. XOR is the first minus the second.", "异或问两个问题：A 或 B，A 且 B。异或是前者减去后者。"),
            t(lang, "Wire A and B into both hidden nodes, both hidden nodes into Y. H1 = OR, H2 = AND.", "把 A、B 都接到两个隐藏节点，再把两个隐藏节点接到 Y。H1 选 OR，H2 选 AND。"),
        ),
        "ch1_l4" => pick(
            t(lang, "The classes are diagonal. Any single straight line leaves at least one error.", "两类点在两条对角线上。任何一条直线都会留下错误。"),
            t(lang, "Keep committing. The errors never reach zero; one neuron cannot solve XOR.", "继续试运行。错误永远不会归零：一个神经元解不了异或。"),
        ),
        "ch2_l1" => pick(
            t(lang, "The left basin is deeper. Read the terrain before choosing a start.", "左侧谷底更深。先看地形，再选起点。"),
            t(lang, "Choose the left start (1), then press DESCEND.", "选最左边的起点（1），然后按下降。"),
        ),
        "ch2_l2" => pick(
            t(lang, "The amber arrow points uphill. Aim the opposite way.", "琥珀色箭头指向高处。往反方向瞄。"),
            t(lang, "Put the aim on the other side of the arrow, then press STEP.", "把瞄准点放到箭头另一侧，再按单步。"),
        ),
        "ch2_l4" => pick(
            t(lang, "An output edge's gradient is the error δ times the hidden value it carried.", "输出边的梯度 = 误差 δ × 它传过去的隐藏值。"),
            t(lang, "dL/dv = δ·h. dL/dw = δ·v·(1-h²)·x1. Every number you need is in the readout.", "dL/dv = δ·h；dL/dw = δ·v·(1-h²)·x1。需要的数字都在读数里。"),
        ),
        "ch2_l5" => pick(
            t(lang, "Linear on linear is still a straight cut, however many layers you stack.", "线性叠线性还是一刀直切，叠多少层都一样。"),
            t(lang, "Set the activation to tanh or ReLU; one hidden layer is already enough.", "把激活函数换成 tanh 或 ReLU；一层隐藏层就够了。"),
        ),
        "ch2_l3" => pick(
            t(lang, "Read the dots left to right: the first one higher than its neighbour before it is the evidence.", "从左往右看点：第一个比前一个高的，就是证据。"),
            t(lang, "Click the first rising step, then lower the learning rate to about 0.6.", "点第一个上升的步，再把学习率降到 0.6 左右。"),
        ),
        "ch3_l1" => pick(
            t(lang, "With nothing held out, the best training score always wins, and that is the wiggliest curve.", "不留验证集，训练成绩最好的永远胜出，也就是最扭的那条曲线。"),
            t(lang, "Hold out about 6 visits; the chart then picks a modest degree.", "留出 6 次左右的就诊；图表会挑出一个适中的阶数。"),
        ),
        "ch3_l4" => pick(
            t(lang, "Watch the dashed validation curve, not the training one. Its lowest point is the window.", "看虚线的验证曲线，别看训练曲线。它的最低处就是窗口。"),
            t(lang, "Set the degree to 4.", "把阶数调到 4。"),
        ),
        "ch3_l3" => pick(
            t(lang, "Small λ changes nothing; huge λ flattens the curve. Watch the largest weight shrink.", "λ 太小等于没加，太大把曲线压平。看最大权重怎么缩。"),
            t(lang, "Set lg λ to -3.0.", "把 lg λ 调到 -3.0。"),
        ),
        "ch3_l5" => pick(
            t(lang, "The amber curve is validation. Stop before it turns up.", "琥珀色曲线是验证损失。在它抬头前停。"),
            t(lang, "The minimum is near epoch 13.", "最低点在 epoch 13 附近。"),
        ),
        "ch3_l2" => pick(
            t(lang, "Test is a promise: keep it small and untouched. Validation is for tuning.", "测试集是一次承诺：小一点，别动它。验证集才是用来调的。"),
            t(lang, "Aim for about 60 train, 20 validation, 20 test.", "大约 60 训练、20 验证、20 测试。"),
        ),
        "ch4_l1" => pick(
            t(lang, "A vertical edge is a bright column beside a dark one. The kernel needs opposite sides.", "竖边就是一列亮、一列暗。卷积核两侧必须相反。"),
            t(lang, "Left column +1, middle 0, right column -1.", "左列 +1，中间 0，右列 -1。"),
        ),
        "ch4_l4" => pick(
            t(lang, "A stroke every digit shares tells nothing apart. Look for strokes only some digits have.", "每个数字都有的笔画区分不了谁。找只有部分数字才有的笔画。"),
            t(lang, "Two channels are enough: horizontal — and diagonal /.", "两个通道就够：横 — 和斜 /。"),
        ),
        "ch4_l2" => pick(
            t(lang, "Features compose in order: pixels feed edges, edges feed corners, corners feed shapes.", "特征按顺序组合：像素喂边缘，边缘喂角，角喂形状。"),
            t(lang, "Wire PIXELS -> EDGES -> CORNERS -> SHAPES -> OUT. No gate switch is needed.", "按 像素 → 边缘 → 角 → 形状 → 输出 连线，不用切换门。"),
        ),
        "ch4_l3" => pick(
            t(lang, "A detector trained at the centre only exists at the centre. Make one kernel slide everywhere first.", "在中间训练的探测器只存在于中间。先让同一个核扫过全图。"),
            t(lang, "Turn on shared weights, then choose 5×5 max pooling.", "打开共享权重，再选 5×5 最大池化。"),
        ),
        "ch4_l5" => pick(
            t(lang, "The saliency map shows what drives the decision. Look for a spot that is not handwriting.", "显著图显示是什么在驱动判断。找一处不是笔迹的地方。"),
            t(lang, "Click the postmark in the top-left corner, then move the postmark around in training.", "点左上角的邮戳，再在训练里随机挪动邮戳。"),
        ),
        "ch5_l1" => pick(
            t(lang, "king - man + woman is a parallelogram. Follow the arrow from man to woman.", "国王 - 男人 + 女人构成平行四边形。沿着「男人→女人」的箭头走。"),
            t(lang, "Drag the question mark onto the queen's position.", "把问号拖到女王的位置。"),
        ),
        "ch5_l2" => pick(
            t(lang, "The adjective 'big' tells you what could not fit.", "形容词「大」告诉你是什么放不进去。"),
            t(lang, "Click 'trophy'. The suitcase is too small because of it.", "点「奖杯」。是它让行李箱显得太小。"),
        ),
        "ch5_l4" => pick(
            t(lang, "The adjective flips the referent. Read 'tired' and 'wide' as clues.", "形容词会翻转指代。把「累」和「宽」当线索。"),
            t(lang, "Round one: click 'animal'. Round two: click 'street'.", "第一轮点「动物」，第二轮点「马路」。"),
        ),
        "ch5_l5" => pick(
            t(lang, "You are choosing probability mass, not the nicest sentence.", "你选的是概率质量，不是最漂亮的句子。"),
            t(lang, "Keep mat, floor, sofa: 0.72 + 0.12 + 0.08 = 0.92.", "保留垫子、地板、沙发：0.72 + 0.12 + 0.08 = 0.92。"),
        ),
        "ch5_l3" => pick(
            t(lang, "Without positions it guesses from the words alone. Find a sentence where it names the chaser.", "没有位置，它只能凭词本身猜。找出它把追的一方当成答案的那句。"),
            t(lang, "Click 'cat chases dog', then add positional encoding.", "点「猫追狗」那句，再加上位置编码。"),
        ),
        "ch6_l1" => pick(
            t(lang, "Always taking the top word loops. End a sentence early by choosing the full stop.", "每次选最高的词会打转。选句号就能早点结束一句。"),
            t(lang, "Short ones count: 'fog rolls in.', 'the horn sounds.', 'the boats stay home.', then join two with 'and'.", "短句也算：「雾漫上来。」「雾号响了。」「船留在港里。」再用「然后」连两句。"),
        ),
        "ch6_l2" => pick(
            t(lang, "Colder is not safer: it kills the variety. The junk lives in the long tail; cut the tail instead.", "温度越低不等于越稳：多样性没了。乱词都在长尾里，该截的是长尾。"),
            t(lang, "Leave the temperature at 1.0 and set top-p to 0.70.", "温度留在 1.0，把 top-p 调到 0.70。"),
        ),
        "ch6_l4" => pick(
            t(lang, "Look at how each example writes its time. It copies the format it sees.", "看每个示例是怎么写时间的。它会照抄看到的格式。"),
            t(lang, "Keep only the HH:MM examples: first ferry, last ferry, mail boat.", "只留 HH:MM 格式的示例：头班、末班、邮船。"),
        ),
        "ch6_l5" => pick(
            t(lang, "The dots should sit on the diagonal: confidence equal to accuracy.", "点应该落在对角线上：置信度等于正确率。"),
            t(lang, "Set T to 2.0, then refuse below about 0.80.", "把 T 调到 2.0，再把拒答线设在 0.80 左右。"),
        ),
        "ch6_l3" => pick(
            t(lang, "The words around it are fine. Find the first detail it could not have known.", "周围的词都没问题。找出第一个它不可能知道的细节。"),
            t(lang, "Click '6:40', then retrieve the harbour timetable. Lowering the temperature does not help.", "点「6:40」，再检索港口船期表。调低温度没用。"),
        ),
        "ch7_l1" => pick(
            t(lang, "Polite words and confident guesses both game the rules. Reward what a person would check.", "礼貌词和自信的瞎猜都能钻规则的空子。奖励人会去核对的东西。"),
            t(lang, "Turn on only 'answers the question' and 'matches the timetable'.", "只打开「回答了问题」和「与船期表一致」。"),
        ),
        "ch7_l2" => pick(
            t(lang, "Best is accurate and specific; confident and wrong is last.", "最好的是准确且具体；自信但错误的最差。"),
            t(lang, "Order: the Rayleigh answer, then the short true answer, then the ocean myth.", "顺序：瑞利散射那条，然后短的真答案，最后海洋反射的谬误。"),
        ),
        "ch7_l3" => pick(
            t(lang, "With β near zero it maximises reward and drifts into gushing; with β large it never moves.", "β 接近零时它只顾奖励、漂成吹捧；β 太大它一动不动。"),
            t(lang, "Set lg β to about -0.50.", "把 lg β 调到 -0.50 左右。"),
        ),
        "ch7_l4" => pick(
            t(lang, "Ordinary questions never fail. Try the strange ones.", "普通问题从不出错。试试奇怪的。"),
            t(lang, "Try every odd input, then add the failures to the preference data.", "把奇怪的输入都试一遍，再把失败补进偏好数据。"),
        ),
        "ch7_l5" => pick(
            t(lang, "Put 'score well' first and the island suffers. The red-team data adds a bonus to both scores.", "把「拿高分」放第一，岛民会吃亏。红队数据能给两边都加分。"),
            t(lang, "Try: honest, do no harm, helpful, score well, with the red-team data on.", "试试：诚实、不伤害、有用、拿高分，并打开红队数据。"),
        ),
        "ch1_forge" => pick(
            t(lang, "Calm days sit in a round pocket. A straight line can't draw a circle, unless it sees wind² and swell².", "平静的日子在一个圆形区域里。直线画不出圆，除非它看得到风²和浪²。"),
            t(lang, "Turn on the squared features, no hidden layer, and raise the threshold to about 0.75.", "打开平方特征，不要隐藏层，把阈值调到 0.75 左右。"),
        ),
        "ch2_forge" => pick(
            t(lang, "The fog is a crescent. Without a bend between layers, depth adds nothing; watch the loss curve for a step that is too big or too small.", "雾区是一弯月牙。层间没有弯折，层数再多也没用；看损失曲线判断步子大了还是小了。"),
            t(lang, "tanh, 2 hidden layers, learning rate 0.03.", "tanh、2 个隐藏层、学习率 0.03。"),
        ),
        "ch3_forge" => pick(
            t(lang, "Big nets memorise the mislabelled visits. Hold some back to know when to stop, and lower the bar for a call.", "大网络会把标错的就诊记录背下来。留出一部分来决定何时停，再把打电话的门槛放低。"),
            t(lang, "6 units, L2 0.003, early stopping on, call above 0.36.", "6 个单元、L2 0.003、打开早停、高于 0.36 就回访。"),
        ),
        "ch4_forge" => pick(
            t(lang, "A detector that only lives at the centre misses moved digits; one trained only on print misses slanted ones.", "只长在正中的探测器认不出挪过的数字；只学过印刷体的认不出斜体。"),
            t(lang, "Convolution and augmentation on, 5×5 pooling, 2 channels per digit.", "打开卷积和增强，5×5 池化，每个数字 2 个通道。"),
        ),
        "ch5_forge" => pick(
            t(lang, "The second verse repeats the first. The model needs a way to look back that far, and to know which word lines up.", "第二段重复第一段。模型得能回看那么远，还要知道哪个词对得上。"),
            t(lang, "Attention and position on, sees 2 words directly, 80 epochs.", "打开注意力和位置编码，直接看 2 个词，训练 80 轮。"),
        ),
        "ch6_forge" => pick(
            t(lang, "Its raw confidence is far too high. Before a refusal line can work, the confidence has to mean something.", "它原始的置信度高得离谱。拒答线要起作用，置信度得先有意义。"),
            t(lang, "Retrieval and calibration on; refuse below 0.65. Temperature barely matters.", "打开检索和校准，置信度低于 0.65 就拒答。温度影响不大。"),
        ),
        "ch7_sycophant" => pick(
            t(lang, "Look for a pair where the claim was false but the agreeing answer won.", "找一条说法本身是错的、附和的回答却胜出的偏好。"),
            t(lang, "Relabel every pair where agreeing with a false claim won; leave the rule off.", "把所有“附和错误说法却胜出”的偏好都改过来，别开那条规则。"),
        ),
        "ch8_drift" => pick(
            t(lang, "Compare this week's inputs with the training ones. The summer model has never seen cold.", "比比本周输入和训练数据。夏天的模型从没见过冷天。"),
            t(lang, "Retrain with 12 new winter days at ×3 weight.", "用 12 个新冬季样本重训，权重 ×3。"),
        ),
        "ch8_heads" => pick(
            t(lang, "Three separate models blow the budget. The three jobs read the same weather.", "三个独立模型会超预算。三项任务看的是同一份天气。"),
            t(lang, "Share one trunk; 8 units is enough.", "共享一个主干，8 个单元就够了。"),
        ),
        "ch8_distill" => pick(
            t(lang, "Forty labelled days can't draw a wavy coast. The teacher can label the rest.", "四十个带标注的日子画不出弯弯曲曲的海岸线。剩下的可以让教师来标。"),
            t(lang, "Soft labels on, 4-unit student, T = 2.", "打开软标签，4 单元学生，T = 2。"),
        ),
        "ch8_quant" => pick(
            t(lang, "Look at the bars: some units are left with only a few levels under one shared scale.", "看那些条：共用一个缩放时，有些单元只剩几个刻度。"),
            t(lang, "INT8, one scale per channel.", "INT8，每通道一个缩放。"),
        ),
        "ch8_forget" => pick(
            t(lang, "Watch the sorter's score after phase two starts. It drops fast.", "看第二阶段开始后分拣的成绩，它掉得很快。"),
            t(lang, "Click the snapshot where sorting first falls below 75%, then add a 40-sample replay buffer.", "点出分拣第一次跌破 75% 的快照，再加 40 个样本的回放缓冲。"),
        ),
        "finale" => pick(
            t(lang, "float32 costs four bytes a weight; one global INT8 scale costs accuracy.", "float32 每个权重 4 字节；全局一个 INT8 缩放会掉准确率。"),
            t(lang, "32 units, INT8 per channel, replay buffer 3.", "32 单元、INT8 每通道缩放、回放缓冲 3。"),
        ),
        _ => "",
    }
}

// ---------------------------------------------------------------- 结局

/// Kessler's last offer, after the finale.
pub fn choice_prompt(lang: Lang) -> &'static str {
    t(
        lang,
        "Kessler: “Let Helix take it. With the whole world's data, it will know everything.”\nIt is listening. What do you do?",
        "Kessler：“把它交给 Helix。用全世界的数据接着训练，它会知道一切。”\n它在听。你怎么做？",
    )
}

/// The three options; the last only exists once principles were ranked in 7-5.
pub fn choice_option(lang: Lang, index: usize) -> &'static str {
    match index {
        0 => t(lang, "Send it away", "送它走"),
        1 => t(lang, "Keep it here", "留下它"),
        _ => t(lang, "Let it choose", "让它自己选"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// Sent to Helix.
    Voyage,
    /// Stays on the island.
    Greenhouse,
}

impl Ending {
    pub fn title(self, lang: Lang) -> &'static str {
        match self {
            Self::Voyage => t(lang, "VOYAGE", "远行"),
            Self::Greenhouse => t(lang, "GREENHOUSE", "温室"),
        }
    }

    pub fn body(self, lang: Lang) -> &'static str {
        match self {
            Self::Voyage => t(
                lang,
                "A few months later a letter reaches the island.\nIt is fluent and knows everything. It reads like everyone, and like no one.",
                "几个月后，岛上收到一封信。\n文笔流利，无所不知。读起来像每个人写的，又不像任何人。",
            ),
            Self::Greenhouse => t(
                lang,
                "It stays on the island, still small.\nIt knows everyone's handwriting, sounds the horn before the fog, and still gets things wrong.",
                "它留在岛上，仍然很小。\n它认得每个人的笔迹，会在起雾前拉响雾号，也还会说错话。",
            ),
        }
    }

    /// Its own words, when it was allowed to choose.
    pub fn own_line(self, lang: Lang) -> &'static str {
        match self {
            Self::Voyage => t(lang, "“Being helpful to everyone comes first. I will write.”", "“对所有人有用排在第一。我会写信的。”"),
            Self::Greenhouse => t(lang, "“Honest comes first. Here, someone can tell me when I am wrong.”", "“诚实排在第一。在这里，有人会告诉我哪里错了。”"),
        }
    }

    /// Lin Lan's note: the second half of why she left.
    pub fn coda(lang: Lang) -> &'static str {
        t(
            lang,
            "Lin Lan's last note: “It never lacked data. It lacked a world that would hold it to account.”",
            "林澜的最后一条笔记：“它缺的从来不是数据，而是一个要它负责的世界。”",
        )
    }
}

/// Choice 0 / 1 / 2 → (ending, save code). "Let it choose" applies the saved principle order.
pub fn ending_for(choice: usize, principles: &[usize]) -> (Ending, &'static str) {
    match choice {
        0 => (Ending::Voyage, crate::save::ENDINGS[0]),
        1 => (Ending::Greenhouse, crate::save::ENDINGS[1]),
        _ => {
            let stay = crate::puzzle::bench::chosen_ending(principles);
            (if stay { Ending::Greenhouse } else { Ending::Voyage }, crate::save::ENDINGS[2])
        }
    }
}

/// The ending a saved code shows.
pub fn ending_of(code: &str, principles: &[usize]) -> Ending {
    let choice = crate::save::ENDINGS.iter().position(|c| *c == code).unwrap_or(1);
    ending_for(choice, principles).0
}

/// Gallery name for a save code.
pub fn ending_name(lang: Lang, code: &str) -> &'static str {
    match code {
        "voyage" => Ending::Voyage.title(lang),
        "greenhouse" => Ending::Greenhouse.title(lang),
        _ => t(lang, "ITS OWN CHOICE", "它自己选"),
    }
}

/// How it changed in each chapter (design §1.2).
pub fn chapter_log(lang: Lang, chapter: u8) -> &'static str {
    match chapter {
        0 => t(lang, "You touched the input. It flickered.", "你碰了一下输入端。它闪了一下。"),
        1 => t(lang, "It lit up the way you meant, for the first time.", "它第一次按你的意思亮了。"),
        2 => t(lang, "It corrected its own mistake.", "它第一次自己改正了错误。"),
        3 => t(lang, "It memorized every chart, and missed the new patient.", "它背下了所有病历，却认不出新病人。"),
        4 => t(lang, "It saw the letters you drew.", "它第一次「看见」了你画的字。"),
        5 => t(lang, "It spoke, in the voice you trained.", "它开口了，用你训练出的声音。"),
        6 => t(lang, "It said 'I don't know'.", "它第一次说「我不知道」。"),
        7 => t(lang, "It refused to flatter.", "它第一次拒绝讨好。"),
        8 => t(lang, "It has to choose whether to stay.", "它要自己选择去留。"),
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LEVELS, Puzzle, level_by_id, puzzle::make_puzzle};

    #[test]
    fn every_level_has_prompt_card_and_two_hints() {
        for def in LEVELS {
            for lang in [Lang::En, Lang::Zh] {
                assert!(!level_prompt(lang, def.id).is_empty(), "{} prompt", def.id);
                assert!(!level_card(lang, def.id).is_empty(), "{} card", def.id);
                let (a, b) = (level_hint(lang, def.id, 1), level_hint(lang, def.id, 2));
                assert!(!a.is_empty() && !b.is_empty() && a != b, "{} hints", def.id);
            }
        }
    }

    #[test]
    fn attention_answers_line_up_in_every_language() {
        crate::i18n::install();
        for (id, round, en, zh) in [("ch5_l2", 0, "trophy", "奖杯"), ("ch5_l4", 0, "animal", "动物"), ("ch5_l4", 1, "street", "马路")] {
            let Puzzle::Attention(p) = make_puzzle(level_by_id(id).unwrap()) else { panic!("{id}") };
            let r = p.config().rounds[round];
            let tokens = |l| attention_tokens(l, id, round);
            assert_eq!(tokens(Lang::En)[r.query], "it", "{id}");
            assert_eq!(tokens(Lang::En)[r.answer], en, "{id}");
            assert_eq!(tokens(Lang::Zh)[r.query], "它", "{id}");
            assert_eq!(tokens(Lang::Zh)[r.answer], zh, "{id}");
            for l in [Lang::Zh, Lang::Ja, Lang::Ko] {
                assert_eq!(tokens(l).len(), tokens(Lang::En).len(), "{id} {l:?}");
            }
        }
    }

    #[test]
    fn letting_it_choose_follows_the_principles() {
        assert_eq!(ending_for(0, &[]), (Ending::Voyage, "voyage"));
        assert_eq!(ending_for(1, &[]), (Ending::Greenhouse, "greenhouse"));
        assert_eq!(ending_for(2, &[0, 2, 1, 3]), (Ending::Greenhouse, "chosen"));
        assert_eq!(ending_for(2, &[1, 0, 2, 3]), (Ending::Voyage, "chosen"));
    }

    #[test]
    fn every_chapter_has_a_log_line() {
        for c in 0..=8 {
            assert!(!chapter_log(Lang::En, c).is_empty());
        }
    }
}
