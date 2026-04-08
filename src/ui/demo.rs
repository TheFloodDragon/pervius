//! 演示数据：文件树 / 搜索结果 / 标签页 / 高亮代码
//!
//! 后续接入真实 JAR 解析后删除。
//!
//! @author sky

use super::editor::highlight::{highlight_java, CodeLine};
use super::editor::tab::TabInfo;
use super::explorer::search::SearchResult;
use super::explorer::tree::TreeNode;
use crate::shell::codicon;

pub fn tree_nodes() -> Vec<TreeNode> {
    vec![
        TreeNode {
            label: "minecraft-server.jar".into(),
            indent: 0,
            is_folder: true,
            is_expanded: true,
            icon: codicon::PACKAGE,
        },
        TreeNode {
            label: "net.minecraft".into(),
            indent: 1,
            is_folder: true,
            is_expanded: true,
            icon: codicon::FOLDER_OPENED,
        },
        TreeNode {
            label: "server".into(),
            indent: 2,
            is_folder: true,
            is_expanded: true,
            icon: codicon::FOLDER_OPENED,
        },
        TreeNode {
            label: "MinecraftServer.class".into(),
            indent: 3,
            is_folder: false,
            is_expanded: false,
            icon: codicon::SYMBOL_CLASS,
        },
        TreeNode {
            label: "ServerLevel.class".into(),
            indent: 3,
            is_folder: false,
            is_expanded: false,
            icon: codicon::SYMBOL_CLASS,
        },
        TreeNode {
            label: "ServerPlayer.class".into(),
            indent: 3,
            is_folder: false,
            is_expanded: false,
            icon: codicon::SYMBOL_CLASS,
        },
        TreeNode {
            label: "level".into(),
            indent: 2,
            is_folder: true,
            is_expanded: false,
            icon: codicon::FOLDER,
        },
        TreeNode {
            label: "world".into(),
            indent: 2,
            is_folder: true,
            is_expanded: false,
            icon: codicon::FOLDER,
        },
        TreeNode {
            label: "network".into(),
            indent: 2,
            is_folder: true,
            is_expanded: false,
            icon: codicon::FOLDER,
        },
        TreeNode {
            label: "commands".into(),
            indent: 2,
            is_folder: true,
            is_expanded: false,
            icon: codicon::FOLDER,
        },
        TreeNode {
            label: "com.mojang".into(),
            indent: 1,
            is_folder: true,
            is_expanded: false,
            icon: codicon::FOLDER,
        },
        TreeNode {
            label: "org.bukkit".into(),
            indent: 1,
            is_folder: true,
            is_expanded: false,
            icon: codicon::FOLDER,
        },
    ]
}

pub fn search_results() -> Vec<SearchResult> {
    vec![
        SearchResult {
            file_path: "net/minecraft/server/MinecraftServer".into(),
            line_num: 142,
            preview: "public void startServer() {".into(),
        },
        SearchResult {
            file_path: "net/minecraft/server/MinecraftServer".into(),
            line_num: 89,
            preview: "private final Thread serverThread;".into(),
        },
        SearchResult {
            file_path: "net/minecraft/server/ServerLevel".into(),
            line_num: 56,
            preview: "public ServerLevel(MinecraftServer server, ...)".into(),
        },
    ]
}

pub fn tabs() -> Vec<TabInfo> {
    vec![
        TabInfo {
            title: "MinecraftServer".into(),
            is_active: true,
            is_modified: false,
        },
        TabInfo {
            title: "ServerLevel".into(),
            is_active: false,
            is_modified: true,
        },
        TabInfo {
            title: "ServerPlayer".into(),
            is_active: false,
            is_modified: false,
        },
    ]
}

const DEMO_JAVA: &str = r#"package net.minecraft.server;

import java.util.List;
import java.util.ArrayList;

/**
 * The main server class that manages the game loop and world.
 */
public class MinecraftServer {

    private final Thread serverThread;
    private final List<ServerLevel> levels;
    private boolean running;

    // Constructs a new server instance
    public MinecraftServer(Thread thread) {
        this.serverThread = thread;
        this.levels = new ArrayList<>();
        this.running = false;
    }

    @Override
    public void startServer() {
        this.running = true;
        loadWorlds();
        runServer();
    }

    private void loadWorlds() {
        levels.add(new ServerLevel(this, "overworld"));
        levels.add(new ServerLevel(this, "the_nether"));
        levels.add(new ServerLevel(this, "the_end"));
    }

    private void runServer() {
        while (running) {
            tickServer();
        }
    }

    private void tickServer() {
        for (ServerLevel level : levels) {
            level.tick();
        }
    }

    public int getLevelCount() {
        return levels.size();
    }
}"#;

pub fn code_lines() -> Vec<CodeLine> {
    highlight_java(DEMO_JAVA)
}
