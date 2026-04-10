//! 搜索 demo 数据：字节码 + 反编译源码
//!
//! @author sky

use super::category::SearchCategory;
use super::result::{SearchMatch, SearchResultGroup, SourcePreview};

fn lines(src: &[&str]) -> Vec<String> {
    src.iter().map(|s| s.to_string()).collect()
}

fn sp(preview: &str, hl: Vec<(usize, usize)>, src: Vec<String>, line: usize) -> SourcePreview {
    SourcePreview {
        preview: preview.into(),
        highlight_ranges: hl,
        source_lines: src,
        match_line: line,
    }
}

fn bc_load_worlds() -> Vec<String> {
    lines(&[
        "// Method: loadWorlds()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  GETFIELD levels : Ljava/util/List;",
        "  LDC \"overworld\"",
        "  ASTORE 2",
        "  INVOKEVIRTUAL register (Ljava/lang/String;)V",
        "L1",
        "  ALOAD 0",
        "  LDC \"the_nether\"",
        "  ASTORE 3",
        "  INVOKEVIRTUAL register (Ljava/lang/String;)V",
        "L2",
        "  ALOAD 0",
        "  LDC \"the_end\"",
        "  ASTORE 4",
        "  INVOKEVIRTUAL register (Ljava/lang/String;)V",
        "L3",
        "  RETURN",
    ])
}

fn bc_dedicated_init() -> Vec<String> {
    lines(&[
        "// Method: <init>()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  INVOKESPECIAL java/lang/Object.<init> ()V",
        "  ALOAD 0",
        "  LDC \"Starting minecraft server\"",
        "  INVOKESTATIC org/slf4j/LoggerFactory.getLogger ()V",
        "  ALOAD 0",
        "  ICONST_0",
        "  PUTFIELD running : Z",
        "L1",
        "  RETURN",
    ])
}

fn bc_dedicated_init_server() -> Vec<String> {
    lines(&[
        "// Method: initServer()Z",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  LDC \"server.properties\"",
        "  ASTORE 1",
        "  NEW java/io/File",
        "  DUP",
        "  ALOAD 1",
        "  INVOKESPECIAL java/io/File.<init> (Ljava/lang/String;)V",
        "  ASTORE 2",
        "L1",
        "  ALOAD 2",
        "  INVOKEVIRTUAL java/io/File.exists ()Z",
        "  IRETURN",
    ])
}

fn bc_mc_init() -> Vec<String> {
    lines(&[
        "// Method: <init>()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  INVOKESPECIAL java/lang/Object.<init> ()V",
        "  ALOAD 0",
        "  ACONST_NULL",
        "  PUTFIELD serverThread : Ljava/lang/Thread;",
        "  ALOAD 0",
        "  NEW java/util/ArrayList",
        "  DUP",
        "  INVOKESPECIAL java/util/ArrayList.<init> ()V",
        "  PUTFIELD levels : Ljava/util/List;",
        "  ALOAD 0",
        "  ICONST_0",
        "  PUTFIELD running : Z",
        "L1",
        "  RETURN",
    ])
}

fn bc_start_server() -> Vec<String> {
    lines(&[
        "// Method: startServer()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  ICONST_1",
        "  PUTFIELD running : Z",
        "  ALOAD 0",
        "  INVOKEVIRTUAL loadWorlds ()V",
        "  ALOAD 0",
        "  INVOKEVIRTUAL tickServer ()V",
        "L1",
        "  RETURN",
    ])
}

fn bc_tick_server() -> Vec<String> {
    lines(&[
        "// Method: tickServer()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  GETFIELD levels : Ljava/util/List;",
        "  INVOKEINTERFACE java/util/List.iterator ()Ljava/util/Iterator;",
        "  ASTORE 1",
        "L1",
        "  ALOAD 1",
        "  INVOKEINTERFACE java/util/Iterator.hasNext ()Z",
        "  IFEQ L3",
        "  ALOAD 1",
        "  INVOKEINTERFACE java/util/Iterator.next ()Ljava/lang/Object;",
        "  CHECKCAST net/minecraft/server/ServerLevel",
        "  ASTORE 2",
        "  ALOAD 2",
        "  INVOKEVIRTUAL net/minecraft/server/ServerLevel.tick ()V",
        "L2",
        "  GOTO L1",
        "L3",
        "  RETURN",
    ])
}

fn bc_get_level_count() -> Vec<String> {
    lines(&[
        "// Method: getLevelCount()I",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  GETFIELD levels : Ljava/util/List;",
        "  INVOKEINTERFACE java/util/List.size ()I",
        "  IRETURN",
    ])
}

fn bc_game_rules_clinit() -> Vec<String> {
    lines(&[
        "// Method: <clinit>()V",
        "// Access: static",
        "L0",
        "  GETSTATIC RANDOM_TICK_SPEED : LGameRule;",
        "  BIPUSH 20",
        "  INVOKEVIRTUAL setValue (I)V",
        "L1",
        "  GETSTATIC MAX_COMMAND_CHAIN : LGameRule;",
        "  SIPUSH 256",
        "  INVOKEVIRTUAL setValue (I)V",
        "L2",
        "  GETSTATIC DO_FIRE_TICK : LGameRule;",
        "  ICONST_1",
        "  INVOKEVIRTUAL setValue (Z)V",
        "L3",
        "  RETURN",
    ])
}

fn bc_mc_class_decl() -> Vec<String> {
    lines(&[
        "// net.minecraft.server.MinecraftServer",
        "// Access: public abstract",
        "// Superclass: java/lang/Object",
        "// Interfaces: java/lang/Runnable",
        "",
        "// Fields",
        "private final Thread serverThread",
        "private final List levels",
        "private volatile boolean running",
        "",
        "// Methods",
        "public void <init>()",
        "public void startServer()",
        "public void loadWorlds()",
        "public void tickServer()",
        "public int getLevelCount()",
    ])
}

fn bc_server_level_decl() -> Vec<String> {
    lines(&[
        "// net.minecraft.server.ServerLevel",
        "// Access: public",
        "// Superclass: net/minecraft/world/level/Level",
        "",
        "// Fields",
        "private final MinecraftServer server",
        "",
        "// Methods",
        "public void <init>(MinecraftServer)",
        "public void tick()",
        "public MinecraftServer getServer()",
    ])
}

fn dc_load_worlds() -> Vec<String> {
    lines(&[
        "public void loadWorlds() {",
        "    this.levels.clear();",
        "    ServerLevel overworld = new ServerLevel(this, \"overworld\");",
        "    this.register(overworld);",
        "    ServerLevel nether = new ServerLevel(this, \"the_nether\");",
        "    this.register(nether);",
        "    ServerLevel end = new ServerLevel(this, \"the_end\");",
        "    this.register(end);",
        "}",
    ])
}

fn dc_dedicated_init() -> Vec<String> {
    lines(&[
        "public DedicatedServer() {",
        "    super();",
        "    LOGGER.info(\"Starting minecraft server\");",
        "    this.running = false;",
        "}",
    ])
}

fn dc_dedicated_init_server() -> Vec<String> {
    lines(&[
        "public boolean initServer() {",
        "    String path = \"server.properties\";",
        "    File file = new File(path);",
        "    return file.exists();",
        "}",
    ])
}

fn dc_mc_init() -> Vec<String> {
    lines(&[
        "public MinecraftServer() {",
        "    super();",
        "    this.serverThread = null;",
        "    this.levels = new ArrayList<>();",
        "    this.running = false;",
        "}",
    ])
}

fn dc_start_server() -> Vec<String> {
    lines(&[
        "public void startServer() {",
        "    this.running = true;",
        "    this.loadWorlds();",
        "    this.tickServer();",
        "}",
    ])
}

fn dc_tick_server() -> Vec<String> {
    lines(&[
        "public void tickServer() {",
        "    for (ServerLevel level : this.levels) {",
        "        level.tick();",
        "    }",
        "}",
    ])
}

fn dc_get_level_count() -> Vec<String> {
    lines(&[
        "public int getLevelCount() {",
        "    return this.levels.size();",
        "}",
    ])
}

fn dc_game_rules_clinit() -> Vec<String> {
    lines(&[
        "static {",
        "    RANDOM_TICK_SPEED.setValue(20);",
        "    MAX_COMMAND_CHAIN.setValue(256);",
        "    DO_FIRE_TICK.setValue(true);",
        "}",
    ])
}

fn dc_mc_class() -> Vec<String> {
    lines(&[
        "public abstract class MinecraftServer implements Runnable {",
        "",
        "    private final Thread serverThread;",
        "    private final List<ServerLevel> levels;",
        "    private volatile boolean running;",
        "",
        "    public MinecraftServer() { ... }",
        "    public void startServer() { ... }",
        "    public void loadWorlds() { ... }",
        "    public void tickServer() { ... }",
        "    public int getLevelCount() { ... }",
        "}",
    ])
}

fn dc_server_level_class() -> Vec<String> {
    lines(&[
        "public class ServerLevel extends Level {",
        "",
        "    private final MinecraftServer server;",
        "",
        "    public ServerLevel(MinecraftServer server) { ... }",
        "    public void tick() { ... }",
        "    public MinecraftServer getServer() { ... }",
        "}",
    ])
}

pub fn demo_results(category: SearchCategory) -> Vec<SearchResultGroup> {
    match category {
        SearchCategory::Strings => vec![
            SearchResultGroup {
                class_name: "MinecraftServer".into(),
                package: "net.minecraft.server".into(),
                expanded: true,
                matches: vec![
                    SearchMatch {
                        location: "loadWorlds()".into(),
                        //                     01234567890123
                        bytecode: sp("LDC \"overworld\"", vec![(5, 14)], bc_load_worlds(), 5),
                        //                                              0         1         2         3         4         5
                        //                                              0123456789012345678901234567890123456789012345678901234567
                        decompiled: sp(
                            "ServerLevel overworld = new ServerLevel(this, \"overworld\");",
                            vec![(46, 55)],
                            dc_load_worlds(),
                            2,
                        ),
                    },
                    SearchMatch {
                        location: "loadWorlds()".into(),
                        bytecode: sp("LDC \"the_nether\"", vec![(5, 15)], bc_load_worlds(), 10),
                        decompiled: sp(
                            "ServerLevel nether = new ServerLevel(this, \"the_nether\");",
                            vec![(43, 53)],
                            dc_load_worlds(),
                            4,
                        ),
                    },
                    SearchMatch {
                        location: "loadWorlds()".into(),
                        bytecode: sp("LDC \"the_end\"", vec![(5, 12)], bc_load_worlds(), 15),
                        decompiled: sp(
                            "ServerLevel end = new ServerLevel(this, \"the_end\");",
                            vec![(40, 47)],
                            dc_load_worlds(),
                            6,
                        ),
                    },
                ],
            },
            SearchResultGroup {
                class_name: "DedicatedServer".into(),
                package: "net.minecraft.server.dedicated".into(),
                expanded: true,
                matches: vec![
                    SearchMatch {
                        location: "<init>()".into(),
                        bytecode: sp(
                            "LDC \"Starting minecraft server\"",
                            vec![(5, 30)],
                            bc_dedicated_init(),
                            6,
                        ),
                        decompiled: sp(
                            "LOGGER.info(\"Starting minecraft server\");",
                            vec![(12, 39)],
                            dc_dedicated_init(),
                            2,
                        ),
                    },
                    SearchMatch {
                        location: "initServer()".into(),
                        bytecode: sp(
                            "LDC \"server.properties\"",
                            vec![(5, 22)],
                            bc_dedicated_init_server(),
                            4,
                        ),
                        decompiled: sp(
                            "String path = \"server.properties\";",
                            vec![(15, 32)],
                            dc_dedicated_init_server(),
                            1,
                        ),
                    },
                ],
            },
        ],
        SearchCategory::Values => vec![SearchResultGroup {
            class_name: "GameRules".into(),
            package: "net.minecraft.world.level".into(),
            expanded: true,
            matches: vec![
                SearchMatch {
                    location: "<clinit>()".into(),
                    bytecode: sp("BIPUSH 20", vec![(7, 9)], bc_game_rules_clinit(), 4),
                    decompiled: sp(
                        "RANDOM_TICK_SPEED.setValue(20);",
                        vec![(26, 28)],
                        dc_game_rules_clinit(),
                        1,
                    ),
                },
                SearchMatch {
                    location: "<clinit>()".into(),
                    bytecode: sp("SIPUSH 256", vec![(7, 10)], bc_game_rules_clinit(), 8),
                    decompiled: sp(
                        "MAX_COMMAND_CHAIN.setValue(256);",
                        vec![(26, 29)],
                        dc_game_rules_clinit(),
                        2,
                    ),
                },
            ],
        }],
        SearchCategory::ClassReferences => vec![
            SearchResultGroup {
                class_name: "MinecraftServer".into(),
                package: "net.minecraft.server".into(),
                expanded: true,
                matches: vec![
                    SearchMatch {
                        location: "loadWorlds()".into(),
                        bytecode: sp(
                            "NEW net/minecraft/server/ServerLevel",
                            vec![(4, 35)],
                            bc_load_worlds(),
                            5,
                        ),
                        decompiled: sp(
                            "ServerLevel overworld = new ServerLevel(this, \"overworld\");",
                            vec![(28, 39)],
                            dc_load_worlds(),
                            2,
                        ),
                    },
                    SearchMatch {
                        location: "tickServer()".into(),
                        bytecode: sp(
                            "CHECKCAST net/minecraft/server/ServerLevel",
                            vec![(10, 41)],
                            bc_tick_server(),
                            13,
                        ),
                        decompiled: sp(
                            "for (ServerLevel level : this.levels) {",
                            vec![(5, 16)],
                            dc_tick_server(),
                            1,
                        ),
                    },
                ],
            },
            SearchResultGroup {
                class_name: "ServerPlayer".into(),
                package: "net.minecraft.server".into(),
                expanded: false,
                matches: vec![SearchMatch {
                    location: "<init>()".into(),
                    bytecode: sp(
                        "INVOKESPECIAL net/minecraft/server/ServerLevel.<init>",
                        vec![(14, 52)],
                        bc_mc_init(),
                        4,
                    ),
                    decompiled: sp(
                        "this.levels = new ArrayList<>();",
                        vec![(18, 31)],
                        dc_mc_init(),
                        3,
                    ),
                }],
            },
        ],
        SearchCategory::MemberReferences => vec![SearchResultGroup {
            class_name: "MinecraftServer".into(),
            package: "net.minecraft.server".into(),
            expanded: true,
            matches: vec![
                SearchMatch {
                    location: "startServer()".into(),
                    bytecode: sp("PUTFIELD running : Z", vec![(9, 16)], bc_start_server(), 5),
                    decompiled: sp("this.running = true;", vec![(5, 12)], dc_start_server(), 1),
                },
                SearchMatch {
                    location: "startServer()".into(),
                    bytecode: sp(
                        "INVOKEVIRTUAL loadWorlds ()V",
                        vec![(14, 24)],
                        bc_start_server(),
                        7,
                    ),
                    decompiled: sp("this.loadWorlds();", vec![(5, 15)], dc_start_server(), 2),
                },
                SearchMatch {
                    location: "tickServer()".into(),
                    bytecode: sp(
                        "GETFIELD levels : Ljava/util/List;",
                        vec![(9, 15)],
                        bc_tick_server(),
                        4,
                    ),
                    decompiled: sp(
                        "for (ServerLevel level : this.levels) {",
                        vec![(29, 35)],
                        dc_tick_server(),
                        1,
                    ),
                },
            ],
        }],
        SearchCategory::MemberDeclarations => vec![
            SearchResultGroup {
                class_name: "MinecraftServer".into(),
                package: "net.minecraft.server".into(),
                expanded: true,
                matches: vec![
                    SearchMatch {
                        location: "field".into(),
                        bytecode: sp(
                            "private final Thread serverThread",
                            vec![(21, 33)],
                            bc_mc_class_decl(),
                            6,
                        ),
                        decompiled: sp(
                            "private final Thread serverThread;",
                            vec![(21, 33)],
                            dc_mc_class(),
                            2,
                        ),
                    },
                    SearchMatch {
                        location: "field".into(),
                        bytecode: sp(
                            "private final List levels",
                            vec![(20, 26)],
                            bc_mc_class_decl(),
                            7,
                        ),
                        decompiled: sp(
                            "private final List<ServerLevel> levels;",
                            vec![(32, 38)],
                            dc_mc_class(),
                            3,
                        ),
                    },
                    SearchMatch {
                        location: "method".into(),
                        bytecode: sp(
                            "public void startServer()",
                            vec![(12, 23)],
                            bc_mc_class_decl(),
                            12,
                        ),
                        decompiled: sp(
                            "public void startServer() { ... }",
                            vec![(12, 23)],
                            dc_mc_class(),
                            7,
                        ),
                    },
                ],
            },
            SearchResultGroup {
                class_name: "ServerLevel".into(),
                package: "net.minecraft.server".into(),
                expanded: true,
                matches: vec![SearchMatch {
                    location: "method".into(),
                    bytecode: sp(
                        "public void tick()",
                        vec![(12, 16)],
                        bc_server_level_decl(),
                        9,
                    ),
                    decompiled: sp(
                        "public void tick() { ... }",
                        vec![(12, 16)],
                        dc_server_level_class(),
                        5,
                    ),
                }],
            },
        ],
        SearchCategory::Instructions => vec![SearchResultGroup {
            class_name: "MinecraftServer".into(),
            package: "net.minecraft.server".into(),
            expanded: true,
            matches: vec![
                SearchMatch {
                    location: "<init>()".into(),
                    bytecode: sp(
                        "INVOKESPECIAL java/lang/Object.<init> ()V",
                        vec![(0, 13)],
                        bc_mc_init(),
                        4,
                    ),
                    decompiled: sp("super();", vec![(0, 7)], dc_mc_init(), 1),
                },
                SearchMatch {
                    location: "loadWorlds()".into(),
                    bytecode: sp(
                        "INVOKEVIRTUAL register (Ljava/lang/String;)V",
                        vec![(0, 13)],
                        bc_load_worlds(),
                        7,
                    ),
                    decompiled: sp(
                        "this.register(overworld);",
                        vec![(5, 13)],
                        dc_load_worlds(),
                        3,
                    ),
                },
                SearchMatch {
                    location: "getLevelCount()".into(),
                    bytecode: sp(
                        "INVOKEINTERFACE java/util/List.size ()I",
                        vec![(0, 15)],
                        bc_get_level_count(),
                        5,
                    ),
                    decompiled: sp(
                        "return this.levels.size();",
                        vec![(18, 22)],
                        dc_get_level_count(),
                        1,
                    ),
                },
            ],
        }],
    }
}
