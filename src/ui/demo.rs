//! 演示数据：文件树 / 搜索结果 / 编辑器标签
//!
//! 后续接入真实 JAR 解析后删除。
//!
//! @author sky

use super::editor::highlight::Language;
use super::editor::EditorTab;
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
            icon: codicon::JAVA,
        },
        TreeNode {
            label: "ServerLevel.class".into(),
            indent: 3,
            is_folder: false,
            is_expanded: false,
            icon: codicon::JAVA,
        },
        TreeNode {
            label: "ServerPlayer.class".into(),
            indent: 3,
            is_folder: false,
            is_expanded: false,
            icon: codicon::JAVA,
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

const DEMO_BYTECODE: &str = r#"// class version 61.0 (Java 17)
// access flags 0x21
public class net/minecraft/server/MinecraftServer {

  // access flags 0x12
  private final Ljava/lang/Thread; serverThread

  // access flags 0x12
  private final Ljava/util/List; levels
  // signature Ljava/util/List<Lnet/minecraft/server/ServerLevel;>;

  // access flags 0x2
  private Z running

  // access flags 0x1
  public <init>(Ljava/lang/Thread;)V
    ALOAD 0
    INVOKESPECIAL java/lang/Object.<init> ()V
    ALOAD 0
    ALOAD 1
    PUTFIELD net/minecraft/server/MinecraftServer.serverThread : Ljava/lang/Thread;
    ALOAD 0
    NEW java/util/ArrayList
    DUP
    INVOKESPECIAL java/util/ArrayList.<init> ()V
    PUTFIELD net/minecraft/server/MinecraftServer.levels : Ljava/util/List;
    ALOAD 0
    ICONST_0
    PUTFIELD net/minecraft/server/MinecraftServer.running : Z
    RETURN
    MAXSTACK = 3
    MAXLOCALS = 2

  // access flags 0x1
  public startServer()V
    ALOAD 0
    ICONST_1
    PUTFIELD net/minecraft/server/MinecraftServer.running : Z
    ALOAD 0
    INVOKEVIRTUAL net/minecraft/server/MinecraftServer.loadWorlds ()V
    ALOAD 0
    INVOKEVIRTUAL net/minecraft/server/MinecraftServer.runServer ()V
    RETURN
    MAXSTACK = 2
    MAXLOCALS = 1

  // access flags 0x2
  private loadWorlds()V
    ALOAD 0
    GETFIELD net/minecraft/server/MinecraftServer.levels : Ljava/util/List;
    NEW net/minecraft/server/ServerLevel
    DUP
    ALOAD 0
    LDC "overworld"
    INVOKESPECIAL net/minecraft/server/ServerLevel.<init> (Lnet/minecraft/server/MinecraftServer;Ljava/lang/String;)V
    INVOKEINTERFACE java/util/List.add (Ljava/lang/Object;)Z (itf)
    POP
    RETURN
    MAXSTACK = 5
    MAXLOCALS = 1

  // access flags 0x1
  public getLevelCount()I
    ALOAD 0
    GETFIELD net/minecraft/server/MinecraftServer.levels : Ljava/util/List;
    INVOKEINTERFACE java/util/List.size ()I (itf)
    IRETURN
    MAXSTACK = 1
    MAXLOCALS = 1
}"#;

const DEMO_HEX: &str = r#"00000000  CA FE BA BE 00 00 00 3D  00 2A 0A 00 02 00 03 07  |.......=.*......|
00000010  00 04 0C 00 05 00 06 01  00 10 6A 61 76 61 2F 6C  |..........java/l|
00000020  61 6E 67 2F 4F 62 6A 65  63 74 01 00 06 3C 69 6E  |ang/Object...<in|
00000030  69 74 3E 01 00 03 28 29  56 09 00 08 00 09 07 00  |it>...()V.......|
00000040  0A 0C 00 0B 00 0C 01 00  27 6E 65 74 2F 6D 69 6E  |........'net/min|
00000050  65 63 72 61 66 74 2F 73  65 72 76 65 72 2F 4D 69  |ecraft/server/Mi|
00000060  6E 65 63 72 61 66 74 53  65 72 76 65 72 01 00 0C  |necraftServer...|
00000070  73 65 72 76 65 72 54 68  72 65 61 64 01 00 12 4C  |serverThread...L|
00000080  6A 61 76 61 2F 6C 61 6E  67 2F 54 68 72 65 61 64  |java/lang/Thread|
00000090  3B 0A 00 0E 00 03 07 00  0F 01 00 13 6A 61 76 61  |;...........java|
000000A0  2F 75 74 69 6C 2F 41 72  72 61 79 4C 69 73 74 09  |/util/ArrayList.|
000000B0  00 08 00 11 0C 00 12 00  13 01 00 06 6C 65 76 65  |............leve|
000000C0  6C 73 01 00 10 4C 6A 61  76 61 2F 75 74 69 6C 2F  |ls...Ljava/util/|
000000D0  4C 69 73 74 3B 09 00 08  00 15 0C 00 16 00 17 01  |List;...........|
000000E0  00 07 72 75 6E 6E 69 6E  67 01 00 01 5A 0A 00 08  |..running...Z...|
000000F0  00 19 0C 00 1A 00 06 01  00 0A 6C 6F 61 64 57 6F  |..........loadWo|
00000100  72 6C 64 73 0A 00 08 00  1C 0C 00 1D 00 06 01 00  |rlds............|
00000110  09 72 75 6E 53 65 72 76  65 72 07 00 1F 01 00 25  |.runServer.....%|
00000120  6E 65 74 2F 6D 69 6E 65  63 72 61 66 74 2F 73 65  |net/minecraft/se|
00000130  72 76 65 72 2F 53 65 72  76 65 72 4C 65 76 65 6C  |rver/ServerLevel|
00000140  0A 00 1E 00 21 0C 00 05  00 22 01 00 3B 28 4C 6E  |....!...."..;(Ln|
00000150  65 74 2F 6D 69 6E 65 63  72 61 66 74 2F 73 65 72  |et/minecraft/ser|
00000160  76 65 72 2F 4D 69 6E 65  63 72 61 66 74 53 65 72  |ver/MinecraftSer|
00000170  76 65 72 3B 4C 6A 61 76  61 2F 6C 61 6E 67 2F 53  |ver;Ljava/lang/S|
00000180  74 72 69 6E 67 3B 29 56  08 00 24 01 00 09 6F 76  |tring;)V..$.overv|
00000190  65 72 77 6F 72 6C 64 0B  00 26 00 27 07 00 28 0C  |erworld..&.'..(.|
000001A0  00 29 00 2A 01 00 0E 6A  61 76 61 2F 75 74 69 6C  |.).*...java/util|
000001B0  2F 4C 69 73 74 01 00 03  61 64 64 01 00 15 28 4C  |/List...add...(L|"#;

/// 从 demo hex dump 文本反向解析出原始字节
fn parse_demo_hex() -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in DEMO_HEX.lines() {
        // 跳过偏移地址（前 10 个字符："XXXXXXXX  "）
        let hex_part = &line[10..];
        // hex 部分到 "|" 之前
        let hex_end = hex_part.find('|').unwrap_or(hex_part.len());
        for token in hex_part[..hex_end].split_whitespace() {
            if let Ok(b) = u8::from_str_radix(token, 16) {
                bytes.push(b);
            }
        }
    }
    bytes
}

pub fn editor_tabs() -> Vec<EditorTab> {
    let demo_bytes = parse_demo_hex();
    vec![
        EditorTab::new(
            "MinecraftServer",
            DEMO_JAVA,
            DEMO_BYTECODE,
            demo_bytes.clone(),
            Language::Java,
        ),
        EditorTab::new(
            "ServerLevel",
            "// ServerLevel.java\npublic class ServerLevel {\n}",
            "// class version 61.0 (Java 17)\npublic class net/minecraft/server/ServerLevel {\n}",
            vec![0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x3D],
            Language::Java,
        ),
        EditorTab::new(
            "ServerPlayer",
            "// ServerPlayer.java\npublic class ServerPlayer {\n}",
            "// class version 61.0 (Java 17)\npublic class net/minecraft/server/ServerPlayer {\n}",
            vec![0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x3D],
            Language::Java,
        ),
    ]
}
