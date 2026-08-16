// Manner_Web - 可以在 Linux 系统上运行的企业管理系统
// Copyright (C) 2026 Linux-System-0(Github) / 一架在Linux上起飞的A320(Bilibili) <ls0_1@qq.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 任务模块数据模型。任务与财务相互独立：仅共享员工/部门等基础表。

use chrono::NaiveDate;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 任务行。
#[derive(Debug, Serialize, FromRow)]
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub assignee_id: String,
    pub assignee_name: String,
    pub creator_id: String,
    pub creator_name: String,
    /// todo 未完成 | done 已完成。
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub completed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

/// 新建任务请求。
#[derive(Debug, Deserialize)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    /// 负责人 id（可指派他人；不传默认本人）。
    pub assignee_id: Option<String>,
    pub due_date: Option<NaiveDate>,
}

/// 更新任务请求（负责人/标题/说明/截止日期；status 用于标记完成/未完成）。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub assignee_id: Option<String>,
    pub status: Option<String>,
    pub due_date: Option<Option<NaiveDate>>,
}

/// 任务查询参数。
#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    /// todo | done | 空（全部）。
    pub status: Option<String>,
    /// 按负责人过滤（仅 task:view_all 有效）。
    pub assignee_id: Option<String>,
    /// all：全员（需 task:view_all）；mine：仅本人。
    pub scope: Option<String>,
}
