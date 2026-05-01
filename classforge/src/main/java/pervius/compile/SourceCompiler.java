package pervius.compile;

import javax.tools.Diagnostic;
import javax.tools.DiagnosticCollector;
import javax.tools.FileObject;
import javax.tools.ForwardingJavaFileManager;
import javax.tools.JavaCompiler;
import javax.tools.JavaFileManager;
import javax.tools.JavaFileObject;
import javax.tools.SimpleJavaFileObject;
import javax.tools.StandardJavaFileManager;
import javax.tools.StandardLocation;
import javax.tools.ToolProvider;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;

/**
 * Java source compiler bridge backed by the JDK javax.tools compiler.
 *
 * Protocol (stdin, big-endian):
 *   [2B binary-name length][binary-name UTF-8]  e.g. "com/foo/Bar"
 *   [4B source length][source UTF-8]
 *
 * Protocol (stdout, big-endian):
 *   [1B status] 0 = ok, 1 = diagnostics, 2 = no javac
 */
public final class SourceCompiler {
    private static final int STATUS_OK = 0;
    private static final int STATUS_DIAGNOSTICS = 1;
    private static final int STATUS_NO_JAVAC = 2;

    private SourceCompiler() {
    }

    public static void run(String[] args) throws Exception {
        String classpathJar = null;
        String target = null;
        boolean debug = false;
        for (int i = 0; i < args.length; i++) {
            if ("--compile".equals(args[i])) {
                continue;
            }
            if ("--classpath".equals(args[i]) && i + 1 < args.length) {
                classpathJar = args[++i];
            } else if ("--target".equals(args[i]) && i + 1 < args.length) {
                target = args[++i];
            } else if ("--debug".equals(args[i])) {
                debug = true;
            }
        }

        JavaCompiler compiler = ToolProvider.getSystemJavaCompiler();
        if (compiler == null) {
            DataOutputStream dos = new DataOutputStream(System.out);
            dos.writeByte(STATUS_NO_JAVAC);
            dos.flush();
            System.err.println("no system javac (JRE detected)");
            System.exit(3);
            return;
        }

        DataInputStream dis = new DataInputStream(System.in);
        String slashBinaryName = readPrefixedString(dis);
        String source = readPrefixedStringU32(dis);
        String dotBinaryName = slashBinaryName.replace('/', '.');

        DiagnosticCollector<JavaFileObject> diagnostics = new DiagnosticCollector<JavaFileObject>();
        StandardJavaFileManager standard = compiler.getStandardFileManager(diagnostics, null, StandardCharsets.UTF_8);
        try (CompileFileManager fileManager = new CompileFileManager(standard, classpathJar)) {
            JavaFileObject sourceObject = new SourceFileObject(dotBinaryName, source);
            List<String> options = buildOptions(target, debug);
            Boolean ok = compiler.getTask(
                    null,
                    fileManager,
                    diagnostics,
                    options,
                    null,
                    Collections.singletonList(sourceObject)
            ).call();

            DataOutputStream dos = new DataOutputStream(System.out);
            if (Boolean.TRUE.equals(ok)) {
                dos.writeByte(STATUS_OK);
                Map<String, byte[]> outputs = fileManager.outputs();
                dos.writeInt(outputs.size());
                for (Map.Entry<String, byte[]> entry : outputs.entrySet()) {
                    writePrefixedString(dos, entry.getKey().replace('.', '/'));
                    byte[] bytes = entry.getValue();
                    dos.writeInt(bytes.length);
                    dos.write(bytes);
                }
            } else {
                dos.writeByte(STATUS_DIAGNOSTICS);
                List<Diagnostic<? extends JavaFileObject>> list = diagnostics.getDiagnostics();
                dos.writeInt(list.size());
                for (Diagnostic<? extends JavaFileObject> diagnostic : list) {
                    dos.writeByte(severity(diagnostic));
                    dos.writeInt(safeInt(diagnostic.getLineNumber()));
                    dos.writeInt(safeInt(diagnostic.getColumnNumber()));
                    writePrefixedStringU32(dos, diagnostic.getMessage(null));
                }
            }
            dos.flush();
        }
    }

    private static List<String> buildOptions(String target, boolean debug) {
        List<String> options = new ArrayList<String>();
        options.add(debug ? "-g" : "-g:none");
        if (target != null && !target.isEmpty()) {
            if (supportsReleaseFlag()) {
                options.add("--release");
                options.add(target);
            } else {
                options.add("-source");
                options.add(target);
                options.add("-target");
                options.add(target);
            }
        }
        return options;
    }

    private static boolean supportsReleaseFlag() {
        String spec = System.getProperty("java.specification.version", "8");
        try {
            if (spec.startsWith("1.")) {
                return Integer.parseInt(spec.substring(2)) >= 9;
            }
            return Integer.parseInt(spec) >= 9;
        } catch (NumberFormatException ignored) {
            return true;
        }
    }

    private static byte severity(Diagnostic<? extends JavaFileObject> diagnostic) {
        Diagnostic.Kind kind = diagnostic.getKind();
        if (kind == Diagnostic.Kind.ERROR) return 0;
        if (kind == Diagnostic.Kind.WARNING || kind == Diagnostic.Kind.MANDATORY_WARNING) return 1;
        return 2;
    }

    private static int safeInt(long value) {
        if (value < 0) return 0;
        if (value > Integer.MAX_VALUE) return Integer.MAX_VALUE;
        return (int) value;
    }

    private static String readPrefixedString(DataInputStream dis) throws IOException {
        int len = dis.readUnsignedShort();
        byte[] buf = new byte[len];
        dis.readFully(buf);
        return new String(buf, StandardCharsets.UTF_8);
    }

    private static String readPrefixedStringU32(DataInputStream dis) throws IOException {
        int len = dis.readInt();
        byte[] buf = new byte[len];
        dis.readFully(buf);
        return new String(buf, StandardCharsets.UTF_8);
    }

    private static void writePrefixedString(DataOutputStream dos, String s) throws IOException {
        byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
        dos.writeShort(bytes.length);
        dos.write(bytes);
    }

    private static void writePrefixedStringU32(DataOutputStream dos, String s) throws IOException {
        byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
        dos.writeInt(bytes.length);
        dos.write(bytes);
    }

    private static final class SourceFileObject extends SimpleJavaFileObject {
        private final String source;

        SourceFileObject(String binaryName, String source) {
            super(URI.create("string:///" + binaryName.replace('.', '/') + Kind.SOURCE.extension), Kind.SOURCE);
            this.source = source;
        }

        @Override
        public CharSequence getCharContent(boolean ignoreEncodingErrors) {
            return source;
        }
    }

    private static final class ByteArrayJavaFileObject extends SimpleJavaFileObject {
        private final ByteArrayOutputStream out = new ByteArrayOutputStream();

        ByteArrayJavaFileObject(String binaryName, Kind kind) {
            super(URI.create("mem:///" + binaryName.replace('.', '/') + kind.extension), kind);
        }

        @Override
        public OutputStream openOutputStream() {
            return out;
        }

        byte[] bytes() {
            return out.toByteArray();
        }
    }

    private static final class JarClassFileObject extends SimpleJavaFileObject {
        private final JarFile jar;
        private final JarEntry entry;
        private final String binaryName;

        JarClassFileObject(JarFile jar, JarEntry entry) {
            super(URI.create("jar://classpath/" + entry.getName()), Kind.CLASS);
            this.jar = jar;
            this.entry = entry;
            String name = entry.getName();
            this.binaryName = name.substring(0, name.length() - Kind.CLASS.extension.length()).replace('/', '.');
        }

        @Override
        public InputStream openInputStream() throws IOException {
            return jar.getInputStream(entry);
        }

        String binaryName() {
            return binaryName;
        }
    }

    private static final class CompileFileManager extends ForwardingJavaFileManager<StandardJavaFileManager> {
        private final JarFile jar;
        private final Map<String, ByteArrayJavaFileObject> outputs = new LinkedHashMap<String, ByteArrayJavaFileObject>();

        CompileFileManager(StandardJavaFileManager fileManager, String classpathJar) throws IOException {
            super(fileManager);
            this.jar = classpathJar == null ? null : new JarFile(classpathJar);
        }

        @Override
        public Iterable<JavaFileObject> list(
                Location location,
                String packageName,
                Set<JavaFileObject.Kind> kinds,
                boolean recurse
        ) throws IOException {
            Iterable<JavaFileObject> base = super.list(location, packageName, kinds, recurse);
            if (jar == null || location != StandardLocation.CLASS_PATH || !kinds.contains(JavaFileObject.Kind.CLASS)) {
                return base;
            }
            List<JavaFileObject> result = new ArrayList<JavaFileObject>();
            for (JavaFileObject object : base) {
                result.add(object);
            }
            String prefix = packageName.isEmpty() ? "" : packageName.replace('.', '/') + "/";
            java.util.Enumeration<JarEntry> entries = jar.entries();
            while (entries.hasMoreElements()) {
                JarEntry entry = entries.nextElement();
                if (entry.isDirectory()) continue;
                String name = entry.getName();
                if (!name.endsWith(JavaFileObject.Kind.CLASS.extension)) continue;
                if (!name.startsWith(prefix)) continue;
                String rest = name.substring(prefix.length());
                if (!recurse && rest.indexOf('/') >= 0) continue;
                result.add(new JarClassFileObject(jar, entry));
            }
            return result;
        }

        @Override
        public String inferBinaryName(Location location, JavaFileObject file) {
            if (file instanceof JarClassFileObject) {
                return ((JarClassFileObject) file).binaryName();
            }
            return super.inferBinaryName(location, file);
        }

        @Override
        public JavaFileObject getJavaFileForOutput(
                Location location,
                String className,
                JavaFileObject.Kind kind,
                FileObject sibling
        ) {
            ByteArrayJavaFileObject out = new ByteArrayJavaFileObject(className, kind);
            outputs.put(className, out);
            return out;
        }

        Map<String, byte[]> outputs() {
            Map<String, byte[]> result = new LinkedHashMap<String, byte[]>();
            for (Map.Entry<String, ByteArrayJavaFileObject> entry : outputs.entrySet()) {
                result.put(entry.getKey(), entry.getValue().bytes());
            }
            return result;
        }

        @Override
        public void close() throws IOException {
            try {
                super.close();
            } finally {
                if (jar != null) {
                    jar.close();
                }
            }
        }
    }
}
