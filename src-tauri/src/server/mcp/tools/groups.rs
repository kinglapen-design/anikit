//! 工具：四批写工具一个（PRD 08-06 四批，2026-10-08 用户定）——set_folder_groups。
//! 场次文件夹「提示词 / P1 / P2」规则组的 Agent 侧入口（此前只能 APP 内人工配）。
//! annotations(read_only_hint = false) + write_gate 门禁；撤销口径：进撤销（复用
//! commands::operations 抽出的 set_folder_groups_op 整段，锁 library → 锁 undo →
//! 单写点 + 单撤销步，与 APP 命令同一实现）。
//! 2026-09-19 PRD「不需要 assign_group 工具」结论维持：自由组成员归组不设独立工具，
//! 经本工具 members 全量管理（全量提交天然覆盖增删）。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, schemars, tool, tool_router};
use tauri::Manager;

use crate::commands::library::AppState;
use crate::core::models::{GroupDef, GroupRule, new_group_id};
use crate::server::mcp::{AnikitMcp, json_result, write_gate};

/// set_folder_groups 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetFolderGroupsParams {
  /// 目标文件夹 id（取自 list_folder_tree 节点 id；anikit://folder/{id} 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id")]
  pub folder_id: String,
  /// 分组定义**全量数组**（草稿式整交 = 该文件夹分组终态；空数组 = 取消分组）。
  /// 改既有组照 list_folder_tree 回包 groups 抄改；漏交的组 = 删除
  pub groups: Vec<GroupDefParams>,
}

/// 分组定义（与 list_folder_tree 回包 GroupDef 同构）：规则组 = name + rules（非空）；
/// 自由组 = 空 rules + members（手动归组，两态不混）
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupDefParams {
  /// 组 id：改既有组必填（照回包抄）；缺省 / 空串 = 新建组，后端生成 id 随回包返回
  pub id: Option<String>,
  /// 组名
  pub name: String,
  /// 规则集（组内多规则 AND 全命中才归入；空数组 = 自由组）。kind 六种：
  /// ext = 扩展名多选（小写不含点，如 ["mp4","png"]）；
  /// type = 类型多选（image 图片 / anim 动图 / video 视频 / audio 音频 / text 文本 / doc 文档 / project 工程 / other 其他）；
  /// name = 名称词列表（词间 AND，匹配名称词干、不含扩展名，不区分大小写）；
  /// importDate / modifyDate = 日期分档多选（档间 OR：today / yesterday / earlierThisWeek / lastWeek / earlierThisMonth / earlier）；
  /// tags = 标签词列表（词间 AND，精确匹配）
  #[serde(default)]
  pub rules: Vec<GroupRuleParams>,
  /// 自由组成员（assetId 有序列表；仅自由组有意义，规则组勿传）
  pub members: Option<Vec<String>>,
}

/// 分组规则：kind + values 值集（values 空数组后端拒绝——自由组请整个 rules 留空）
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GroupRuleParams {
  /// 规则种类：ext / type / name / importDate / modifyDate / tags
  pub kind: String,
  /// 值集（取值见 GroupDefParams.rules 说明）
  #[serde(default)]
  pub values: Vec<String>,
}

#[tool_router(router = groups_router, vis = "pub")]
impl AnikitMcp {
  /// 设置文件夹分组定义（全量草稿式提交 = 该文件夹分组终态，空数组 = 取消分组）；
  /// 返回终态 GroupDef 数组（新建组含后端生成的 id，后续 assign / 再编辑凭它定位）。
  /// 归组是显示层计算、不动资产数据；多组同时命中归命中关键字更多者。进撤销
  #[tool(name = "set_folder_groups", annotations(read_only_hint = false))]
  fn set_folder_groups(&self, Parameters(p): Parameters<SetFolderGroupsParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let groups: Vec<GroupDef> = p
      .groups
      .into_iter()
      .map(|g| GroupDef {
        id: g.id.filter(|s| !s.trim().is_empty()).unwrap_or_else(new_group_id),
        name: g.name,
        rules: g.rules.into_iter().map(|r| GroupRule { kind: r.kind, values: r.values }).collect(),
        members: g.members,
      })
      .collect();
    let state = self.app.state::<AppState>();
    match crate::commands::operations::set_folder_groups_op(&state, &p.folder_id, groups.clone()) {
      Ok(()) => {
        self.emit_library_changed("set_folder_groups");
        json_result(&groups)
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }
}
