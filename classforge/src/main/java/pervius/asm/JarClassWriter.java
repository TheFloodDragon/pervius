package pervius.asm;

import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;

import java.io.Closeable;
import java.io.IOException;
import java.io.InputStream;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Set;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;

/**
 * 支持从 JAR 解析类层次的 ClassWriter
 *
 * 覆写 getCommonSuperClass，直接从 JAR 读取 class 字节解析继承链，
 * 不依赖 ClassLoader 加载目标类，避免副作用和版本冲突。
 *
 * @author sky
 */
public class JarClassWriter extends ClassWriter implements Closeable {

    /// 源 JAR（用于读取应用类）
    private final JarFile jarFile;
    /// 父类缓存（type → superName），避免重复读取
    private final Map<String, String> superCache = new HashMap<>();

    public JarClassWriter(ClassReader reader, int flags, String jarPath) throws IOException {
        super(reader, flags);
        this.jarFile = jarPath != null ? new JarFile(jarPath) : null;
    }

    public JarClassWriter(int flags, String jarPath) throws IOException {
        super(flags);
        this.jarFile = jarPath != null ? new JarFile(jarPath) : null;
    }

    @Override
    protected String getCommonSuperClass(String type1, String type2) {
        if (type1.equals(type2)) {
            return type1;
        }
        // 收集 type1 的完整祖先链
        Set<String> ancestors = new LinkedHashSet<>();
        String t = type1;
        while (t != null && ancestors.add(t)) {
            if ("java/lang/Object".equals(t)) break;
            t = resolveSuperClass(t);
        }
        // 沿 type2 祖先链向上，找到第一个交集
        t = type2;
        while (t != null) {
            if (ancestors.contains(t)) return t;
            if ("java/lang/Object".equals(t)) break;
            t = resolveSuperClass(t);
        }
        return "java/lang/Object";
    }

    /**
     * 解析指定类的直接父类
     *
     * 优先从 JAR 读取 class 字节用 ClassReader 解析，
     * 回退到系统 ClassLoader（覆盖 JDK 类）。
     */
    private String resolveSuperClass(String type) {
        String cached = superCache.get(type);
        if (cached != null) {
            return cached;
        }
        String superName = resolveFromJar(type);
        if (superName == null) {
            superName = resolveFromClassLoader(type);
        }
        if (superName == null) {
            superName = "java/lang/Object";
        }
        superCache.put(type, superName);
        return superName;
    }

    /**
     * 从 JAR 文件读取 class 字节，解析父类名
     */
    private String resolveFromJar(String type) {
        if (jarFile == null) return null;
        try {
            JarEntry entry = jarFile.getJarEntry(type + ".class");
            if (entry == null) return null;
            try (InputStream is = jarFile.getInputStream(entry)) {
                return new ClassReader(is).getSuperName();
            }
        } catch (IOException e) {
            return null;
        }
    }

    /**
     * 从系统 ClassLoader 加载类，获取父类名
     */
    private String resolveFromClassLoader(String type) {
        try {
            Class<?> c = Class.forName(type.replace('/', '.'), false, getClass().getClassLoader());
            if (c.isInterface()) return "java/lang/Object";
            Class<?> sc = c.getSuperclass();
            return sc != null ? sc.getName().replace('.', '/') : null;
        } catch (ClassNotFoundException e) {
            return null;
        }
    }

    @Override
    public void close() throws IOException {
        if (jarFile != null) {
            jarFile.close();
        }
    }
}
