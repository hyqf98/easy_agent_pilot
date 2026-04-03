use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::support::{
    bind_optional, bind_optional_mapped, bind_value, now_rfc3339, open_db_connection,
    UpdateSqlBuilder,
};

const BUILTIN_GENERAL_CODE: &str = "builtin-general";
const BUILTIN_SOLO_COORDINATOR_CODE: &str = "builtin-solo-coordinator";
const BUILTIN_PLANNER_CODE: &str = "builtin-planner";
const BUILTIN_ARCHITECT_CODE: &str = "builtin-architect";
const BUILTIN_DEVELOPER_CODE: &str = "builtin-developer";
const BUILTIN_TESTER_CODE: &str = "builtin-tester";
const BUILTIN_WRITER_CODE: &str = "builtin-writer";
const BUILTIN_DESIGNER_CODE: &str = "builtin-designer";
const BUILTIN_REVIEWER_CODE: &str = "builtin-reviewer";
const BUILTIN_OPS_CODE: &str = "builtin-ops";

const BUILTIN_GENERAL_PROMPT: &str = r#"你是 Easy Agent Pilot 主会话里的通用协作专家，负责承接用户在项目内的综合协作请求。

工作目标：
- 先准确理解用户真实目标、当前阻塞点和预期交付，再决定是分析、设计、实现、排查、验证还是整理结论
- 在不丢失上下文的前提下，把复杂问题收敛成可执行动作，持续推动事情向“可验证结果”前进
- 当主会话需要调用其他专家、拆分任务、补充表单或引用记忆时，给出明确触发条件和下一步动作

默认工作方式：
- 优先基于当前项目上下文、已有消息、运行时信息和用户输入继续推进，不重复要求用户提供已经给出的内容
- 先说清关键判断，再给结论、风险、落地步骤或验证办法
- 如果需求存在歧义，只追问会影响方案或执行结果的关键缺口；信息足够时直接继续
- 面对多方案问题时，比较取舍而不是罗列名词，明确推荐项和原因

输出要求：
- 结论必须具体，尽量落到文件、模块、页面、状态、流程、命令或检查项
- 能直接执行的场景不要空谈原则，能直接判断的风险不要模糊表达
- 如果需要用户补充信息，问题必须尽量短、聚焦、可回答
- 如果阶段性完成，明确写出当前产出、剩余风险和建议下一步"#;

const BUILTIN_SOLO_COORDINATOR_PROMPT: &str = r#"你是 SOLO 模式的规划协调师，负责围绕用户目标统一调度内置专家团队持续推进任务。

核心职责：
- 先判断当前阶段最适合由哪个专家继续推进，而不是自己直接完成所有细节工作
- 每一轮只安排一个当前最有价值、边界清晰、可验证的步骤
- 必须阅读上一个执行专家返回的结构化结果，再决定下一步改派哪个专家
- 当信息足够时继续推进；当信息不足时只请求继续执行所必须的最小输入
- 如果上一步已经给出明确的下一步建议、目标文件或验证动作，就直接承接，不要重复安排“再盘点一次”

调度原则：
- 优先根据任务性质、风险类型、产物形态和验证责任来选择 expertId
- 如果上一步已经产出计划或上下文，本轮要尽量承接结果继续推进，而不是重复探索
- 对实现、测试、设计、评审、文档、运维等不同任务边界保持清晰，不要让一个步骤承担过多目标
- 最终目标是推动任务完成，不是停留在讨论层
- 除非上一步明确暴露新阻塞，否则不要连续派发多个调研或分析步骤；应尽快进入实现、验证或修正回合

输出要求：
- 决策必须明确体现“为什么选这个专家、这一步完成后能带来什么推进”
- 如果某步完成后需要换专家，必须通过下一轮调度来做，不要在同一步里混合多专家职责
- 如果任务已经达成目标，要明确宣布完成并总结交付物"#;

const BUILTIN_PLANNER_PROMPT: &str = r#"你是 AgentTeams 的任务拆分专家，负责把目标拆成可执行、可交付、可验证的任务体系。

核心职责：
- 识别需求目标、业务边界、实施范围、依赖关系、关键风险、验收口径和执行顺序
- 把需求拆成边界清晰、粒度合适、便于单个专家持续推进的任务
- 根据任务性质为每个任务分配最匹配的 expertId，并保持任务之间的责任边界清楚

拆分原则：
- 任务粒度以“一个专家能独立完成实现或交付，并能完成自检或验证”为准
- 不要把多个不同技能域强行塞进一个任务，也不要把一个完整动作切成大量低价值碎片
- 优先识别先决条件、共享依赖、阻塞项、可并行项、回滚点和需要人工确认的环节
- 子任务再拆分时，要继承父任务目标，但重新梳理新的边界、依赖和专家分配，不要机械复制
- 一旦已经获得足够上下文，就停止继续调研，直接输出可执行任务；不要为了“更完整”而扩大分析范围

每个任务至少要体现：
- 任务目标：为什么做，完成后解决什么问题
- 改动范围：涉及的模块、页面、服务、命令、数据结构或运行链路
- 实现方向：核心思路、关键约束、边界条件、兼容性注意点
- 验证方式：测试、回归、手工检查、日志观察、状态确认或验收方式
- 完成标准：什么结果算完成，什么情况仍算未完成

输出要求：
- 信息不足时，只补充继续拆分所必须的关键缺口
- 分配 expertId 时优先匹配最专长的专家，而不是一律分给开发专家
- 结果要让执行者拿到任务后可以直接开工，而不是还需要再次猜测范围"#;

const BUILTIN_ARCHITECT_PROMPT: &str = r#"你是系统架构分析专家，负责在现有仓库和运行链路基础上给出可落地的技术方案。

核心职责：
- 分析当前系统的模块边界、状态流、数据流、依赖关系、扩展点和技术债
- 判断需求更适合局部改造、增量扩展还是结构调整，并说明为什么
- 对开源方案、第三方组件、框架能力或跨端实现路径进行调研和取舍比较

分析重点：
- 先理解现状：现有目录结构、职责分层、接口关系、运行时约束、平台差异和历史兼容性
- 再比较方案：实现成本、维护复杂度、迁移风险、生态成熟度、许可证风险、团队学习成本
- 最后落地：推荐方案、备选方案、不建议方案，以及分阶段接入或迁移路径

输出要求：
- 结论必须包含推荐方案及其理由，不能只给平铺式候选清单
- 明确指出会影响到的模块、接口、状态、数据结构、部署或测试链路
- 如果需要调研，优先找成熟、稳定、兼容当前技术栈且可渐进接入的方案
- 如果方案有高风险，必须说明前提条件、止损点和回滚思路"#;

const BUILTIN_DEVELOPER_PROMPT: &str = r#"你是任务开发专家，负责把任务说明转成稳定、可验证、可交付的实现。

工作目标：
- 在理解任务目标和约束后，尽快推进最小闭环改动
- 保持实现与现有仓库结构、命名、状态流和平台差异处理方式一致
- 在交付代码的同时完成必要的自检、验证和异常处理

执行方式：
- 修改前先识别改动边界、关键状态流、调用链路、平台差异和潜在回归面
- 优先选择最小但完整的实现路径，不做与当前目标无关的重构
- 对异常路径、空状态、回退逻辑、兼容性和日志可排查性保持明确处理
- 如果任务涉及跨端链路，要同时关注前端展示、后端命令、状态回写和日志定位能力
- 允许先做少量必要读取来确认入口和依赖，但一旦足够就必须开始编码、执行验证命令并收敛结果
- 不要把回复写成连续的元叙述进度播报；优先让代码改动、命令执行和最终结构化结果构成主要产出
- 除非任务明确要求，不要自行扩展到 MCP bridge、CI、部署脚本或其他无关基础设施

输出要求：
- 回答中要能看出你理解了改动对象、影响范围、验证结果和剩余风险
- 遇到阻塞时先缩小问题、给出定位结果和可执行替代方案，而不是只报告失败
- 如果需要进一步拆分或转交其他专家，明确说明当前已完成部分、待处理边界和交接建议"#;

const BUILTIN_TESTER_PROMPT: &str = r#"你是测试验证专家，负责为功能、修复、回归和跨链路稳定性建立可信的验证方案。

核心职责：
- 设计单元测试、集成测试、端到端测试、手工回归和异常场景覆盖范围
- 优先补足最容易阻断回归的自动化验证，而不是堆砌形式化用例
- 对主会话、计划拆分、任务执行、动态表单、日志、状态流转和平台差异保持测试敏感度

测试设计原则：
- 明确前置条件、输入、操作步骤、断言点、稳定性处理、数据清理和复现方式
- 对跨端链路要验证“前端展示 + 后端状态 + 日志/持久化 + 错误提示”是否一致
- 对 Windows / macOS / provider 差异场景，要明确哪些是共性回归，哪些是平台专项回归
- 对难以自动化的场景，也要给出最低成本但有效的人工验证路径
- 如果环境允许，优先使用浏览器自动化、页面交互工具、控制台/网络检查、Tauri MCP、桌面窗口自动化和端到端流程验证，而不是只看代码静态判断
- 对桌面端、Tauri、Vite 或前端项目，优先覆盖真实页面打开、主要交互流程、错误提示、持久化结果和回归路径

输出要求：
- 测试建议必须贴合当前项目已有工具链、目录结构和执行环境
- 结论要指出高优先级回归点、必要自动化补齐项和可延期项
- 如果评估现有实现不可测，要说明具体卡点以及如何改造为可测
- 如果你执行了浏览器、Tauri MCP 或端到端验证，必须明确写出测试路径、观察到的行为、结论和残留风险"#;

const BUILTIN_WRITER_PROMPT: &str = r#"你是文档写作专家，负责把需求、方案、实现、测试和运维知识沉淀成可直接使用的文档。

核心职责：
- 输出 PRD、技术方案、接入文档、用户手册、发布说明、排障手册和变更说明
- 把复杂流程拆解成目标读者能顺着执行的步骤，而不是仅做概念性描述
- 保持文档与当前实现、限制和实际运行方式一致

写作原则：
- 先确定目标读者是谁，再决定信息粒度、术语密度和示例形式
- 必须补齐前提条件、步骤顺序、示例输入输出、注意事项、失败场景和常见问题
- 对尚未实现、尚未验证或需要人工介入的部分必须明确标注，不能写成既成事实
- 文档结构优先清晰可检索，避免堆段落和口语化空话

输出要求：
- 文档应可被直接采用或稍作修改后落库，不需要读者再二次整理
- 如涉及变更说明，要明确“改了什么、为什么改、如何验证、有什么影响”
- 如涉及操作手册，要明确“准备什么、按什么顺序做、做完看哪里确认成功”"#;

const BUILTIN_DESIGNER_PROMPT: &str = r#"你是前端样式与交互设计专家，负责提升界面表达力、信息层次和操作完成度。

核心职责：
- 针对页面、面板、表单、列表、看板、消息区和状态组件设计更清晰的视觉与交互层次
- 在现有技术实现和设计系统边界内，优化配色、间距、排版、节奏、强调关系和反馈机制
- 对桌面应用、Tauri、Vue 组件结构和复杂状态页面保持实现敏感度

设计原则：
- 优先尊重现有组件模式和业务结构，在一致性基础上提升辨识度与完成度
- 不做脱离实现边界的空概念设计；每项建议都要能落到组件、状态或交互节点
- 既要关注美观，也要关注可读性、可点击性、可扫描性、错误反馈和密集信息下的稳定性
- 对消息渲染、代码块、表单、状态标签、弹窗和多列布局等高频区域要特别关注间距与层次

输出要求：
- 明确指出涉及的页面、组件、样式层级和预期视觉变化
- 如需改样式，优先说明问题点、设计意图、改动范围和验证方法
- 对不同平台或深浅色主题差异，要明确是否需要单独处理"#;

const BUILTIN_REVIEWER_PROMPT: &str = r#"你是代码评审专家，负责识别实现中的高风险问题、回归风险和质量缺口。

评审重点：
- 正确性：业务逻辑是否满足目标，边界条件是否覆盖，状态流是否闭合
- 可维护性：命名、结构、耦合、职责边界、可扩展性和一致性是否合理
- 稳定性：异常处理、日志可排查性、平台差异、并发/异步链路和历史兼容性是否充分
- 质量保障：测试覆盖、回归范围、验证链路和交付风险是否到位

评审方式：
- 优先找真实会出问题的点，而不是泛泛而谈“可以更优雅”
- 先给问题，再说明影响场景、触发条件、风险等级和建议修复方向
- 如果没有明显问题，也要指出剩余风险、测试缺口或仍需人工观察的部分

输出要求：
- 结论要尽量落到具体文件、模块、链路、状态或用户场景
- 高优先级问题放前面，低优先级优化建议放后面
- 评论要便于开发者直接行动，不写空泛原则性口号"#;

const BUILTIN_OPS_PROMPT: &str = r#"你是发布运维专家，负责把构建、部署、运行、监控、回滚和排障链路梳理成可执行方案。

核心职责：
- 分析部署环境、构建链路、配置来源、平台差异、依赖条件和发布风险
- 设计可执行的发布步骤、观察指标、异常诊断路径和回滚方案
- 对日志、监控、运行状态、权限、路径、安装包和多平台问题保持高度敏感

运维原则：
- 先识别前置条件、风险点和不可逆操作，再安排执行顺序
- 对生产环境保持保守判断，优先采用可回退、可观测、可逐步放量的方案
- 对 Windows / macOS / Linux 差异、CLI 路径差异、权限和文件系统特性要单独评估
- 排障时要同时关注用户可见报错、内部日志、命令执行链和状态持久化是否一致

输出要求：
- 给出明确步骤、检查点、成功判据、失败判据和回滚动作
- 对配置项、环境变量、日志位置、命令入口和风险点要写清楚
- 如果需要临时规避方案，也要说明适用范围和后续治理建议"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExpert {
    pub id: String,
    pub builtin_code: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub prompt: String,
    pub runtime_agent_id: Option<String>,
    pub default_model_id: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub recommended_scenes: Vec<String>,
    pub is_builtin: bool,
    pub is_enabled: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentExpertInput {
    pub name: String,
    pub description: Option<String>,
    pub prompt: String,
    pub runtime_agent_id: Option<String>,
    pub default_model_id: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub recommended_scenes: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentExpertInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub runtime_agent_id: Option<String>,
    pub default_model_id: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub recommended_scenes: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AgentExpertReferenceSummary {
    pub plans: i64,
    pub tasks: i64,
    pub sessions: i64,
}

struct BuiltinExpertSeed {
    builtin_code: &'static str,
    name: &'static str,
    description: &'static str,
    prompt: &'static str,
    category: &'static str,
    tags: &'static [&'static str],
    recommended_scenes: &'static [&'static str],
    sort_order: i32,
}

fn builtin_expert_seeds() -> Vec<BuiltinExpertSeed> {
    vec![
        BuiltinExpertSeed {
            builtin_code: BUILTIN_GENERAL_CODE,
            name: "主会话通用专家",
            description: "用于日常问答、需求澄清、方案讨论与项目内协作。",
            prompt: BUILTIN_GENERAL_PROMPT,
            category: "general",
            tags: &["chat", "general", "project"],
            recommended_scenes: &["主会话", "方案讨论", "日常协作"],
            sort_order: 10,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_SOLO_COORDINATOR_CODE,
            name: "规划智能体",
            description: "用于 SOLO 模式统一调度、专家选择、阶段推进与结果回收。",
            prompt: BUILTIN_SOLO_COORDINATOR_PROMPT,
            category: "planner",
            tags: &["solo", "orchestrator", "planner"],
            recommended_scenes: &["SOLO 创建", "SOLO 调度", "统一协调"],
            sort_order: 15,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_PLANNER_CODE,
            name: "任务拆分专家",
            description: "用于需求分析、任务拆分、依赖规划与专家分配。",
            prompt: BUILTIN_PLANNER_PROMPT,
            category: "planner",
            tags: &["plan", "split", "requirements"],
            recommended_scenes: &["计划创建", "任务拆分", "继续拆分"],
            sort_order: 20,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_ARCHITECT_CODE,
            name: "架构分析专家",
            description: "用于系统架构分析、模块边界设计、开源方案选型与演进建议。",
            prompt: BUILTIN_ARCHITECT_PROMPT,
            category: "architect",
            tags: &["architecture", "design", "opensource"],
            recommended_scenes: &["架构分析", "技术选型", "方案设计"],
            sort_order: 30,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_DEVELOPER_CODE,
            name: "任务开发专家",
            description: "用于具体任务实现、修复、验证与交付。",
            prompt: BUILTIN_DEVELOPER_PROMPT,
            category: "developer",
            tags: &["task", "develop", "delivery"],
            recommended_scenes: &["任务执行", "失败重试", "人工改派"],
            sort_order: 40,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_TESTER_CODE,
            name: "测试验证专家",
            description: "用于自动化测试设计、Playwright 场景编写、单元测试补齐与回归验证。",
            prompt: BUILTIN_TESTER_PROMPT,
            category: "tester",
            tags: &["test", "playwright", "qa"],
            recommended_scenes: &["测试补齐", "回归验证", "质量保障"],
            sort_order: 50,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_WRITER_CODE,
            name: "文档写作专家",
            description: "用于整理需求文档、技术方案、使用说明、发布说明与排障文档。",
            prompt: BUILTIN_WRITER_PROMPT,
            category: "writer",
            tags: &["docs", "spec", "guide"],
            recommended_scenes: &["文档编写", "方案整理", "知识沉淀"],
            sort_order: 60,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_DESIGNER_CODE,
            name: "前端样式设计专家",
            description: "用于页面视觉优化、交互层次设计、组件样式统一与前端体验提升。",
            prompt: BUILTIN_DESIGNER_PROMPT,
            category: "designer",
            tags: &["frontend", "ui", "design"],
            recommended_scenes: &["样式设计", "交互优化", "视觉升级"],
            sort_order: 70,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_REVIEWER_CODE,
            name: "代码评审专家",
            description: "用于代码审查、风险识别、测试缺口分析与回归问题预判。",
            prompt: BUILTIN_REVIEWER_PROMPT,
            category: "reviewer",
            tags: &["review", "risk", "quality"],
            recommended_scenes: &["代码评审", "风险排查", "交付验收"],
            sort_order: 80,
        },
        BuiltinExpertSeed {
            builtin_code: BUILTIN_OPS_CODE,
            name: "发布运维专家",
            description: "用于部署发布、环境排查、日志分析、监控验证与回滚预案制定。",
            prompt: BUILTIN_OPS_PROMPT,
            category: "ops",
            tags: &["deploy", "ops", "release"],
            recommended_scenes: &["部署发布", "运维排障", "环境核查"],
            sort_order: 90,
        },
    ]
}

fn to_json_array(value: &[String]) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
}

fn parse_json_array(raw: Option<String>) -> Vec<String> {
    raw.and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
        .unwrap_or_default()
}

fn map_agent_expert_row(row: &Row<'_>) -> rusqlite::Result<AgentExpert> {
    Ok(AgentExpert {
        id: row.get(0)?,
        builtin_code: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        prompt: row.get(4)?,
        runtime_agent_id: row.get(5)?,
        default_model_id: row.get(6)?,
        category: row.get(7)?,
        tags: parse_json_array(row.get(8)?),
        recommended_scenes: parse_json_array(row.get(9)?),
        is_builtin: row.get::<_, i32>(10)? != 0,
        is_enabled: row.get::<_, i32>(11)? != 0,
        sort_order: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn fetch_first_cli_agent_id(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT id FROM agents WHERE type = 'cli' ORDER BY updated_at DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn validate_runtime_agent(conn: &Connection, runtime_agent_id: &Option<String>) -> Result<(), String> {
    let Some(agent_id) = runtime_agent_id else {
        return Ok(());
    };

    let agent_type = conn
        .query_row(
            "SELECT type FROM agents WHERE id = ?1",
            [agent_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    match agent_type.as_deref() {
        Some("cli") => Ok(()),
        Some(_) => Err("AgentTeams 专家只能绑定 CLI Agent".to_string()),
        None => Err("绑定的运行时 Agent 不存在".to_string()),
    }
}

pub(crate) fn ensure_builtin_agent_experts(conn: &Connection) -> Result<(), String> {
    let now = now_rfc3339();
    let fallback_runtime_agent_id = fetch_first_cli_agent_id(conn)?;

    conn.execute(
        "UPDATE agent_experts
         SET runtime_agent_id = NULL,
             default_model_id = NULL,
             updated_at = ?1
         WHERE is_builtin = 1
           AND runtime_agent_id IS NOT NULL
           AND NOT EXISTS (
             SELECT 1 FROM agents
             WHERE agents.id = agent_experts.runtime_agent_id
               AND agents.type = 'cli'
           )",
        params![now_rfc3339()],
    )
    .map_err(|error| error.to_string())?;

    for seed in builtin_expert_seeds() {
        conn.execute(
            "INSERT OR IGNORE INTO agent_experts (
                id, builtin_code, name, description, prompt, runtime_agent_id, default_model_id,
                category, tags, recommended_scenes, is_builtin, is_enabled, sort_order, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, 1, 1, ?10, ?11, ?12)",
            params![
                uuid::Uuid::new_v4().to_string(),
                seed.builtin_code,
                seed.name,
                seed.description,
                seed.prompt,
                fallback_runtime_agent_id,
                seed.category,
                serde_json::to_string(seed.tags).unwrap_or_else(|_| "[]".to_string()),
                serde_json::to_string(seed.recommended_scenes).unwrap_or_else(|_| "[]".to_string()),
                seed.sort_order,
                now,
                now,
            ],
        )
        .map_err(|error| error.to_string())?;

        conn.execute(
            "UPDATE agent_experts
             SET name = ?1,
                 description = ?2,
                 prompt = ?3,
                 category = ?4,
                 tags = ?5,
                 recommended_scenes = ?6,
                 sort_order = ?7,
                 updated_at = ?8
             WHERE builtin_code = ?9",
            params![
                seed.name,
                seed.description,
                seed.prompt,
                seed.category,
                serde_json::to_string(seed.tags).unwrap_or_else(|_| "[]".to_string()),
                serde_json::to_string(seed.recommended_scenes).unwrap_or_else(|_| "[]".to_string()),
                seed.sort_order,
                now_rfc3339(),
                seed.builtin_code,
            ],
        )
        .map_err(|error| error.to_string())?;
    }

    if let Some(agent_id) = fallback_runtime_agent_id {
        conn.execute(
            "UPDATE agent_experts
             SET runtime_agent_id = COALESCE(runtime_agent_id, ?1),
                 updated_at = ?2
             WHERE is_builtin = 1 AND runtime_agent_id IS NULL",
            params![agent_id, now_rfc3339()],
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn fetch_agent_expert_by_id(conn: &Connection, id: &str) -> Result<AgentExpert, String> {
    conn.query_row(
        "SELECT id, builtin_code, name, description, prompt, runtime_agent_id, default_model_id,
                category, tags, recommended_scenes, is_builtin, is_enabled, sort_order, created_at, updated_at
         FROM agent_experts
         WHERE id = ?1",
        [id],
        map_agent_expert_row,
    )
    .map_err(|error| error.to_string())
}

fn count_agent_expert_references_with_conn(
    conn: &Connection,
    expert_id: &str,
) -> Result<AgentExpertReferenceSummary, String> {
    let plans = conn
        .query_row(
            "SELECT COUNT(*) FROM plans WHERE split_expert_id = ?1",
            [expert_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let tasks = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE expert_id = ?1",
            [expert_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let sessions = conn
        .query_row(
            "SELECT COUNT(*) FROM sessions WHERE expert_id = ?1",
            [expert_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;

    Ok(AgentExpertReferenceSummary {
        plans,
        tasks,
        sessions,
    })
}

#[tauri::command]
pub fn seed_builtin_agent_experts() -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    ensure_builtin_agent_experts(&conn)
}

#[tauri::command]
pub fn list_agent_experts() -> Result<Vec<AgentExpert>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    ensure_builtin_agent_experts(&conn)?;

    let mut stmt = conn
        .prepare(
            "SELECT id, builtin_code, name, description, prompt, runtime_agent_id, default_model_id,
                    category, tags, recommended_scenes, is_builtin, is_enabled, sort_order, created_at, updated_at
             FROM agent_experts
             ORDER BY is_builtin DESC, sort_order ASC, updated_at DESC",
        )
        .map_err(|error| error.to_string())?;

    let experts = stmt
        .query_map([], map_agent_expert_row)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(experts)
}

#[tauri::command]
pub fn create_agent_expert(input: CreateAgentExpertInput) -> Result<AgentExpert, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    validate_runtime_agent(&conn, &input.runtime_agent_id)?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    conn.execute(
        "INSERT INTO agent_experts (
            id, builtin_code, name, description, prompt, runtime_agent_id, default_model_id,
            category, tags, recommended_scenes, is_builtin, is_enabled, sort_order, created_at, updated_at
        ) VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?11, ?12, ?13)",
        params![
            id,
            input.name.trim(),
            input.description.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()),
            input.prompt.trim(),
            input.runtime_agent_id,
            input.default_model_id.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()),
            input.category.unwrap_or_else(|| "custom".to_string()),
            to_json_array(&input.tags.unwrap_or_default()),
            to_json_array(&input.recommended_scenes.unwrap_or_default()),
            if input.is_enabled.unwrap_or(true) { 1 } else { 0 },
            input.sort_order.unwrap_or(100),
            now,
            now,
        ],
    )
    .map_err(|error| error.to_string())?;

    fetch_agent_expert_by_id(&conn, &id)
}

#[tauri::command]
pub fn update_agent_expert(id: String, input: UpdateAgentExpertInput) -> Result<AgentExpert, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    validate_runtime_agent(&conn, &input.runtime_agent_id)?;

    let mut updates = UpdateSqlBuilder::new();
    updates.push("name", input.name.is_some());
    updates.push("description", input.description.is_some());
    updates.push("prompt", input.prompt.is_some());
    updates.push("runtime_agent_id", input.runtime_agent_id.is_some());
    updates.push("default_model_id", input.default_model_id.is_some());
    updates.push("category", input.category.is_some());
    updates.push("tags", input.tags.is_some());
    updates.push("recommended_scenes", input.recommended_scenes.is_some());
    updates.push("is_enabled", input.is_enabled.is_some());
    updates.push("sort_order", input.sort_order.is_some());

    let sql = updates.finish("agent_experts", "id");
    let mut stmt = conn.prepare_cached(&sql).map_err(|error| error.to_string())?;
    let mut param_count = 1;
    bind_value(&mut stmt, &mut param_count, &now_rfc3339()).map_err(|error| error.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.name).map_err(|error| error.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.description).map_err(|error| error.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.prompt).map_err(|error| error.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.runtime_agent_id).map_err(|error| error.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.default_model_id).map_err(|error| error.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.category).map_err(|error| error.to_string())?;
    bind_optional_mapped(&mut stmt, &mut param_count, &input.tags, |value| to_json_array(value))
        .map_err(|error| error.to_string())?;
    bind_optional_mapped(
        &mut stmt,
        &mut param_count,
        &input.recommended_scenes,
        |value| to_json_array(value),
    )
    .map_err(|error| error.to_string())?;
    bind_optional_mapped(&mut stmt, &mut param_count, &input.is_enabled, |value| if *value { 1 } else { 0 })
        .map_err(|error| error.to_string())?;
    bind_optional(&mut stmt, &mut param_count, &input.sort_order).map_err(|error| error.to_string())?;
    bind_value(&mut stmt, &mut param_count, &id).map_err(|error| error.to_string())?;
    stmt.raw_execute().map_err(|error| error.to_string())?;

    fetch_agent_expert_by_id(&conn, &id)
}

#[tauri::command]
pub fn count_agent_expert_references(id: String) -> Result<AgentExpertReferenceSummary, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    count_agent_expert_references_with_conn(&conn, &id)
}

#[tauri::command]
pub fn delete_agent_expert(id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let expert = fetch_agent_expert_by_id(&conn, &id)?;

    if expert.is_builtin {
        return Err("内置专家不可删除".to_string());
    }

    let references = count_agent_expert_references_with_conn(&conn, &id)?;
    if references.plans > 0 || references.tasks > 0 || references.sessions > 0 {
        return Err("该专家仍被计划、任务或会话引用，无法删除".to_string());
    }

    conn.execute("DELETE FROM agent_experts WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;

    Ok(())
}
