-- Manner_Web - 可以在 Linux 系统上运行的企业管理系统
-- Copyright (C) 2026 Linux-System-0(Github) / 一架在Linux上起飞的A320(Bilibili) <ls0_1@qq.com>
--
-- This program is free software: you can redistribute it and/or modify
-- it under the terms of the GNU General Public License as published by
-- the Free Software Foundation, either version 3 of the License, or
-- (at your option) any later version.
--
-- This program is distributed in the hope that it will be useful,
-- but WITHOUT ANY WARRANTY; without even the implied warranty of
-- MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
-- GNU General Public License for more details.
--
-- You should have received a copy of the GNU General Public License
-- along with this program.  If not, see <https://www.gnu.org/licenses/>.

CREATE DATABASE IF NOT EXISTS manner_web CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

USE manner_web;

CREATE TABLE IF NOT EXISTS employees (
    id CHAR(36) NOT NULL PRIMARY KEY,
    username VARCHAR(64) NOT NULL UNIQUE,
    password VARCHAR(255) NOT NULL,
    name VARCHAR(64) NOT NULL,
    title VARCHAR(64) DEFAULT NULL,
    -- 敏感字段：静态加密（AES-256-GCM）后落库，密文带 enc:v1: 前缀，列宽按密文预留
    email VARCHAR(255) DEFAULT NULL,
    phone VARCHAR(255) DEFAULT NULL,
    id_number VARCHAR(255) DEFAULT NULL,
    address TEXT DEFAULT NULL,
    hire_date DATE DEFAULT NULL,
    status TINYINT NOT NULL DEFAULT 1,
    pwd_version INT NOT NULL DEFAULT 0,
    must_change_password TINYINT NOT NULL DEFAULT 0,
    -- 当前有效会话 id（单设备登录：新登录覆盖此值，旧设备令牌立即失效）
    active_session VARCHAR(64) DEFAULT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    avatar VARCHAR(255) DEFAULT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_status (status),
    INDEX idx_name (name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS permissions (
    id INT AUTO_INCREMENT PRIMARY KEY,
    code VARCHAR(64) NOT NULL UNIQUE,
    name VARCHAR(64) NOT NULL,
    module VARCHAR(32) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 权限字典种子（幂等）。chat:protect_block 为防拉黑保护权限码，
-- 由角色授权派生（员工有效权限含该码则不可被拉黑）。
INSERT IGNORE INTO permissions (code, name, module) VALUES
('employee:list',   '查看员工列表',   'employee'),
('employee:view',   '查看员工详情',   'employee'),
('employee:create', '新增员工',       'employee'),
('employee:edit',   '编辑员工',       'employee'),
('employee:delete', '删除员工',       'employee'),
('employee:password', '重置员工密码', 'employee'),
('employee:view_sensitive', '查看敏感信息', 'employee'),
('chat:protect_block', '防拉黑保护', 'chat'),
('chat:group_create', '群聊创建', 'chat'),
('chat:upload', '上传文件', 'chat'),
('system:config',   '系统配置',       'system'),
('system:settings', '系统设置',       'system');

-- 个人偏好列（历史演进遗留的 ALTER，幂等；protect_block 已随权限重构移除）。
ALTER TABLE employees ADD COLUMN preferences TEXT DEFAULT NULL;

-- 敏感字段静态加密后落库：扩列宽以容纳 enc:v1: 密文（MODIFY 幂等，重复执行不报错）
ALTER TABLE employees MODIFY COLUMN email VARCHAR(255) DEFAULT NULL;
ALTER TABLE employees MODIFY COLUMN phone VARCHAR(255) DEFAULT NULL;
ALTER TABLE employees MODIFY COLUMN id_number VARCHAR(255) DEFAULT NULL;
ALTER TABLE employees MODIFY COLUMN address TEXT DEFAULT NULL;

CREATE TABLE IF NOT EXISTS token_blacklist (
    id CHAR(36) NOT NULL PRIMARY KEY,
    jti VARCHAR(255) NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_jti (jti),
    INDEX idx_expires_at (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 员工级直接授权表 employee_permissions 已彻底移除（方案 C 仅走角色授权）。
-- 存量数据库升级时由 main.rs 的 migrate_direct_permissions 迁移数据后删除该表；
-- 新建库不创建此表。

CREATE TABLE IF NOT EXISTS conversations (
    id CHAR(36) NOT NULL PRIMARY KEY,
    type VARCHAR(16) NOT NULL DEFAULT 'single' COMMENT 'single|group',
    name VARCHAR(128) DEFAULT NULL,
    created_by CHAR(36) DEFAULT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS conversation_participants (
    conversation_id CHAR(36) NOT NULL,
    employee_id CHAR(36) NOT NULL,
    joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    role VARCHAR(16) NOT NULL DEFAULT 'member',
    nickname VARCHAR(64) DEFAULT NULL,
    group_note VARCHAR(255) DEFAULT NULL,
    PRIMARY KEY (conversation_id, employee_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS messages (
    id CHAR(36) NOT NULL PRIMARY KEY,
    conversation_id CHAR(36) NOT NULL,
    sender_id CHAR(36) NOT NULL,
    type VARCHAR(16) NOT NULL DEFAULT 'text' COMMENT 'text|file',
    content TEXT,
    file_url VARCHAR(512) DEFAULT NULL,
    file_name VARCHAR(256) DEFAULT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_conversation_time (conversation_id, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS system_settings (
    setting_key VARCHAR(64) NOT NULL PRIMARY KEY,
    setting_value TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- F-07: 首个管理员注册开关（默认开启，注册成功后关闭，防并发抢注）
INSERT IGNORE INTO system_settings (setting_key, setting_value) VALUES ('registration_open', '1');

-- 默认语言包（system=跟随系统/浏览器语言；en-US / zh-CN=手动指定）
INSERT IGNORE INTO system_settings (setting_key, setting_value) VALUES ('default_language', 'en-US');

-- 登录限流参数(可在系统设置界面调整,无需重启,环境变量为兜底默认值)
INSERT IGNORE INTO system_settings (setting_key, setting_value) VALUES ('login_max_failures', '5');
INSERT IGNORE INTO system_settings (setting_key, setting_value) VALUES ('login_lock_window_secs', '900');

CREATE TABLE IF NOT EXISTS blocked_users (
    blocker_id CHAR(36) NOT NULL,
    blocked_id CHAR(36) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (blocker_id, blocked_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

DELETE FROM permissions WHERE code = 'employee:protect_block';
INSERT IGNORE INTO permissions (code, name, module) VALUES ('chat:group_create', '群聊创建', 'chat');
INSERT IGNORE INTO permissions (code, name, module) VALUES ('system:settings', '系统设置', 'system');

-- 部门表：支持父子层级（parent_id）与部门负责人（leader_id，指向 employees.id）。
CREATE TABLE IF NOT EXISTS departments (
    id CHAR(36) NOT NULL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    parent_id CHAR(36) DEFAULT NULL,
    leader_id CHAR(36) DEFAULT NULL,
    sort_order INT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_parent (parent_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 员工-部门多对多归属（一个员工可加入多个部门）。
CREATE TABLE IF NOT EXISTS employee_departments (
    employee_id CHAR(36) NOT NULL,
    department_id CHAR(36) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (employee_id, department_id),
    FOREIGN KEY (employee_id) REFERENCES employees(id) ON DELETE CASCADE,
    FOREIGN KEY (department_id) REFERENCES departments(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 部门负责人多对多（一个部门可有多个负责人）。
CREATE TABLE IF NOT EXISTS department_leaders (
    department_id CHAR(36) NOT NULL,
    employee_id CHAR(36) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (department_id, employee_id),
    FOREIGN KEY (department_id) REFERENCES departments(id) ON DELETE CASCADE,
    FOREIGN KEY (employee_id) REFERENCES employees(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT IGNORE INTO permissions (code, name, module) VALUES
('department:list',   '查看部门列表',   'department'),
('department:view',   '查看部门详情',   'department'),
('department:create', '新增部门',       'department'),
('department:edit',   '编辑部门',       'department'),
('department:delete', '删除部门',       'department');

-- ============================================================================
-- 权限系统重构（方案 C）：RBAC + 数据范围 + 部门角色继承
-- 权限模型：最终权限 = Σ（员工角色 + 部门角色）沿 parent_id 继承链展开后的角色权限并集。
-- 数据范围（roles.scope_type）：all 全部 / subtree 本部门及子树 / department 本部门 /
--                               self 仅本人 / custom 指定部门集合（role_department_scopes）。
-- ============================================================================

-- 员工权限版本号：权限相关变更递增，中间件比对令牌内的版本号，不一致即从库重算
-- 有效权限（实现权限变更即时生效，消除原「最长 30 分钟令牌快照」延迟）。
ALTER TABLE employees ADD COLUMN perm_version INT NOT NULL DEFAULT 0;

-- 存量 protect_block 冗余字段移除：防拉黑改为按目标员工有效权限是否含 chat:protect_block 判定。
-- 注意：新建库无此列时本条会失败，启动流程对该类语句仅告警不中断，属预期。
ALTER TABLE employees DROP COLUMN protect_block;

-- 新增权限码：角色管理（角色 CRUD / 权限分配 / 数据范围设置 / 部门角色绑定）。
INSERT IGNORE INTO permissions (code, name, module) VALUES ('role:manage', '角色管理', 'role');

-- 角色表：支持父子继承（parent_id）与数据范围（scope_type）。
CREATE TABLE IF NOT EXISTS roles (
    id CHAR(36) NOT NULL PRIMARY KEY,
    name VARCHAR(64) NOT NULL UNIQUE,
    parent_id CHAR(36) DEFAULT NULL,
    is_system TINYINT NOT NULL DEFAULT 0,
    scope_type VARCHAR(16) NOT NULL DEFAULT 'self' COMMENT 'all|subtree|department|self|custom',
    description VARCHAR(255) DEFAULT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_parent (parent_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 角色编码（code）已移除：角色以唯一名称 + id 标识（super_admin 用固定 id 特判）。
-- 存量库若仍存在 code 列则删除（新建库无此列，本句失败仅告警，属预期）。
ALTER TABLE roles DROP COLUMN code;

-- 角色-权限 多对多。
CREATE TABLE IF NOT EXISTS role_permissions (
    role_id CHAR(36) NOT NULL,
    permission_id INT NOT NULL,
    PRIMARY KEY (role_id, permission_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 员工-角色 多对多（直接分配）。
CREATE TABLE IF NOT EXISTS employee_roles (
    employee_id CHAR(36) NOT NULL,
    role_id CHAR(36) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (employee_id, role_id),
    FOREIGN KEY (employee_id) REFERENCES employees(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 部门-角色 多对多（部门角色继承：员工归属部门后自动获得该部门绑定的角色权限）。
CREATE TABLE IF NOT EXISTS department_roles (
    department_id CHAR(36) NOT NULL,
    role_id CHAR(36) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (department_id, role_id),
    FOREIGN KEY (department_id) REFERENCES departments(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 角色数据范围明细：scope_type=custom 时指定的部门集合。
CREATE TABLE IF NOT EXISTS role_department_scopes (
    role_id CHAR(36) NOT NULL,
    department_id CHAR(36) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (role_id, department_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (department_id) REFERENCES departments(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 种子：super_admin 内置角色（全权限、all 数据范围，固定 id），首个管理员注册时绑定。
-- is_system=1：不可删除/改名/改权限/改范围；员工角色分配中不允许通过部门角色绑定。
INSERT IGNORE INTO roles (id, name, is_system, scope_type, description)
VALUES ('00000000-0000-0000-0000-000000000001', '超级管理员', 1, 'all', '系统内置超级管理员角色，拥有全部权限，不可删除');

-- super_admin 授予全部权限（幂等，新权限码追加后自动补齐）。
INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000001', id FROM permissions;
