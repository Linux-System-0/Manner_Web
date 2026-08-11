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

CREATE TABLE IF NOT EXISTS token_blacklist (
    id CHAR(36) NOT NULL PRIMARY KEY,
    jti VARCHAR(255) NOT NULL UNIQUE,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_jti (jti),
    INDEX idx_expires_at (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS employee_permissions (
    employee_id CHAR(36) NOT NULL,
    permission_id INT NOT NULL,
    PRIMARY KEY (employee_id, permission_id),
    FOREIGN KEY (employee_id) REFERENCES employees(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

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

ALTER TABLE employees ADD COLUMN protect_block TINYINT NOT NULL DEFAULT 0;
ALTER TABLE employees ADD COLUMN preferences TEXT DEFAULT NULL AFTER protect_block;
ALTER TABLE employees ADD COLUMN active_session VARCHAR(64) DEFAULT NULL;

-- 敏感字段静态加密后落库：扩列宽以容纳 enc:v1: 密文（MODIFY 幂等，重复执行不报错）
ALTER TABLE employees MODIFY COLUMN email VARCHAR(255) DEFAULT NULL;
ALTER TABLE employees MODIFY COLUMN phone VARCHAR(255) DEFAULT NULL;
ALTER TABLE employees MODIFY COLUMN id_number VARCHAR(255) DEFAULT NULL;
ALTER TABLE employees MODIFY COLUMN address TEXT DEFAULT NULL;

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

-- 旧版本角色机制已移除：清理残留权限与废弃表（幂等，可重复执行）。
-- 当前系统为「员工级直接授权 + 部门（多对多归属）」。
DELETE FROM permissions WHERE module = 'role';
DROP TABLE IF EXISTS role_department_scopes;
DROP TABLE IF EXISTS role_permissions;
DROP TABLE IF EXISTS employee_roles;
DROP TABLE IF EXISTS roles;

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
