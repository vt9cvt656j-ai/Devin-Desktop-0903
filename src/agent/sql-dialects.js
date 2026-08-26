/**
 * 各数据库的 SQL 方言表：关键字 / 函数（带说明）/ 常用类型 / 长脚本模板。
 *
 * 从 main.js 抽出来的第九块。判据照旧：纯数据，零外部依赖，没有 DOM、没有模块级
 * 可变状态。它给 SQL 编辑器的补全和高亮用——一张表一件事。
 */

// 各数据库方言分开：关键字 / 函数（带说明）/ 常用类型 / 长脚本模板。别把 SQLite 的
// PRAGMA、MySQL 的 SHOW、PG 的 RETURNING 混着提示。
export const sqlDialects = {
  sqlite: {
    label: "SQLite",
    keywords: ["SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "NULL", "IS", "IS NOT", "IN", "LIKE", "GLOB", "BETWEEN", "ORDER BY", "GROUP BY", "HAVING", "LIMIT", "OFFSET", "DISTINCT", "AS", "ON", "JOIN", "LEFT JOIN", "INNER JOIN", "CROSS JOIN", "UNION", "UNION ALL", "INSERT INTO", "VALUES", "UPDATE", "SET", "DELETE FROM", "CREATE TABLE", "ALTER TABLE", "DROP TABLE", "CREATE INDEX", "CREATE VIEW", "PRIMARY KEY", "FOREIGN KEY", "REFERENCES", "DEFAULT", "AUTOINCREMENT", "UNIQUE", "PRAGMA", "EXPLAIN", "EXPLAIN QUERY PLAN", "BEGIN", "COMMIT", "ROLLBACK", "CASE", "WHEN", "THEN", "ELSE", "END", "ASC", "DESC", "EXISTS", "CAST", "COLLATE", "WITH", "RETURNING", "VACUUM"],
    types: ["INTEGER", "TEXT", "REAL", "BLOB", "NUMERIC", "BOOLEAN"],
    funcs: [
      ["count", "count(X) — 统计行数/非空值数"], ["sum", "sum(X) — 求和"], ["avg", "avg(X) — 平均值"],
      ["min", "min(X) — 最小值"], ["max", "max(X) — 最大值"], ["total", "total(X) — 求和（总返回浮点）"],
      ["abs", "abs(X) — 绝对值"], ["round", "round(X, n) — 四舍五入"], ["length", "length(X) — 字符/字节长度"],
      ["lower", "lower(X) — 转小写"], ["upper", "upper(X) — 转大写"], ["trim", "trim(X) — 去首尾空白"],
      ["ltrim", "ltrim(X) — 去左空白"], ["rtrim", "rtrim(X) — 去右空白"],
      ["coalesce", "coalesce(A, B, …) — 返回第一个非 NULL"], ["ifnull", "ifnull(A, B) — A 为 NULL 时返回 B"],
      ["nullif", "nullif(A, B) — A=B 时返回 NULL"], ["substr", "substr(X, start, len) — 截取子串"],
      ["replace", "replace(X, old, new) — 替换"], ["instr", "instr(X, sub) — 子串位置"],
      ["hex", "hex(X) — 转十六进制"], ["typeof", "typeof(X) — 值类型"], ["random", "random() — 随机整数"],
      ["date", "date('now') — 日期"], ["time", "time('now') — 时间"], ["datetime", "datetime('now', 'localtime') — 日期时间"],
      ["strftime", "strftime('%Y-%m-%d', X) — 格式化时间"], ["julianday", "julianday(X) — 儒略日"],
      ["unixepoch", "unixepoch(X) — Unix 时间戳"], ["group_concat", "group_concat(X, sep) — 拼接分组值"],
      ["json_extract", "json_extract(X, '$.k') — 取 JSON 字段"], ["last_insert_rowid", "last_insert_rowid() — 最后插入的 rowid"],
    ],
    snippets: [
      ["SELECT … FROM … WHERE", "SELECT ${1:*}\nFROM ${2:table}\nWHERE ${3:condition};"],
      ["SELECT … JOIN", "SELECT ${1:*}\nFROM ${2:a}\nJOIN ${3:b} ON ${4:a.id} = ${5:b.a_id}\nWHERE ${6:1 = 1};"],
      ["按列计数", "SELECT ${1:col}, COUNT(*) AS n\nFROM ${2:table}\nGROUP BY ${1:col}\nORDER BY n DESC;"],
      ["INSERT INTO", "INSERT INTO ${1:table} (${2:col})\nVALUES (${3:value});"],
      ["UPDATE … SET", "UPDATE ${1:table}\nSET ${2:col} = ${3:value}\nWHERE ${4:condition};"],
      ["DELETE FROM", "DELETE FROM ${1:table}\nWHERE ${2:condition};"],
      ["CREATE TABLE", "CREATE TABLE ${1:name} (\n  id INTEGER PRIMARY KEY AUTOINCREMENT,\n  ${2:col} ${3:TEXT}\n);"],
      ["PRAGMA 表结构", "PRAGMA table_info(${1:table});"],
    ],
  },
  mysql: {
    label: "MySQL",
    keywords: ["SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "NULL", "IS", "IS NOT", "IN", "LIKE", "BETWEEN", "ORDER BY", "GROUP BY", "HAVING", "LIMIT", "DISTINCT", "AS", "ON", "JOIN", "LEFT JOIN", "INNER JOIN", "RIGHT JOIN", "STRAIGHT_JOIN", "UNION", "UNION ALL", "INSERT INTO", "VALUES", "ON DUPLICATE KEY UPDATE", "REPLACE INTO", "UPDATE", "SET", "DELETE FROM", "CREATE TABLE", "ALTER TABLE", "DROP TABLE", "CREATE INDEX", "PRIMARY KEY", "FOREIGN KEY", "REFERENCES", "DEFAULT", "AUTO_INCREMENT", "UNIQUE KEY", "ENGINE", "SHOW", "SHOW TABLES", "SHOW COLUMNS FROM", "SHOW CREATE TABLE", "DESCRIBE", "EXPLAIN", "USE", "CASE", "WHEN", "THEN", "ELSE", "END", "ASC", "DESC", "EXISTS", "CAST", "WITH", "LOCK", "UNSIGNED"],
    types: ["INT", "BIGINT", "TINYINT", "SMALLINT", "VARCHAR(255)", "CHAR", "TEXT", "LONGTEXT", "DATETIME", "TIMESTAMP", "DATE", "TIME", "DECIMAL(10,2)", "FLOAT", "DOUBLE", "JSON", "ENUM"],
    funcs: [
      ["COUNT", "COUNT(*) — 统计行数"], ["SUM", "SUM(X) — 求和"], ["AVG", "AVG(X) — 平均"], ["MIN", "MIN(X)"], ["MAX", "MAX(X)"],
      ["ABS", "ABS(X)"], ["ROUND", "ROUND(X, n)"], ["LENGTH", "LENGTH(X) — 字节长度"], ["CHAR_LENGTH", "CHAR_LENGTH(X) — 字符数"],
      ["LOWER", "LOWER(X)"], ["UPPER", "UPPER(X)"], ["TRIM", "TRIM(X)"], ["CONCAT", "CONCAT(A, B, …) — 拼接"],
      ["CONCAT_WS", "CONCAT_WS(sep, A, B) — 带分隔符拼接"], ["COALESCE", "COALESCE(A, B) — 第一个非 NULL"],
      ["IFNULL", "IFNULL(A, B)"], ["IF", "IF(cond, a, b) — 条件"], ["NULLIF", "NULLIF(A, B)"],
      ["SUBSTRING", "SUBSTRING(X, pos, len)"], ["REPLACE", "REPLACE(X, old, new)"], ["LOCATE", "LOCATE(sub, X)"],
      ["NOW", "NOW() — 当前日期时间"], ["CURDATE", "CURDATE() — 当前日期"], ["CURTIME", "CURTIME()"],
      ["DATE_FORMAT", "DATE_FORMAT(X, '%Y-%m-%d')"], ["DATE_ADD", "DATE_ADD(X, INTERVAL 1 DAY)"],
      ["DATEDIFF", "DATEDIFF(A, B) — 天数差"], ["UNIX_TIMESTAMP", "UNIX_TIMESTAMP(X)"],
      ["GROUP_CONCAT", "GROUP_CONCAT(X SEPARATOR ',')"], ["JSON_EXTRACT", "JSON_EXTRACT(X, '$.k')"],
    ],
    snippets: [
      ["SELECT … FROM … WHERE", "SELECT ${1:*}\nFROM ${2:table}\nWHERE ${3:condition};"],
      ["SELECT … JOIN", "SELECT ${1:*}\nFROM ${2:a}\nJOIN ${3:b} ON ${4:a.id} = ${5:b.a_id}\nWHERE ${6:1 = 1};"],
      ["INSERT INTO", "INSERT INTO ${1:table} (${2:col})\nVALUES (${3:value});"],
      ["INSERT … ON DUPLICATE", "INSERT INTO ${1:table} (${2:col})\nVALUES (${3:value})\nON DUPLICATE KEY UPDATE ${2:col} = VALUES(${2:col});"],
      ["UPDATE … SET", "UPDATE ${1:table}\nSET ${2:col} = ${3:value}\nWHERE ${4:condition};"],
      ["DELETE FROM", "DELETE FROM ${1:table}\nWHERE ${2:condition};"],
      ["CREATE TABLE", "CREATE TABLE ${1:name} (\n  id BIGINT PRIMARY KEY AUTO_INCREMENT,\n  ${2:col} ${3:VARCHAR(255)}\n) ENGINE=InnoDB;"],
      ["SHOW CREATE TABLE", "SHOW CREATE TABLE ${1:table};"],
    ],
  },
  postgres: {
    label: "PostgreSQL",
    keywords: ["SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "NULL", "IS", "IS NOT", "IN", "LIKE", "ILIKE", "BETWEEN", "ORDER BY", "GROUP BY", "HAVING", "LIMIT", "OFFSET", "DISTINCT", "DISTINCT ON", "AS", "ON", "JOIN", "LEFT JOIN", "INNER JOIN", "RIGHT JOIN", "FULL JOIN", "LATERAL", "UNION", "UNION ALL", "INSERT INTO", "VALUES", "ON CONFLICT", "DO NOTHING", "DO UPDATE SET", "RETURNING", "UPDATE", "SET", "DELETE FROM", "CREATE TABLE", "ALTER TABLE", "DROP TABLE", "CREATE INDEX", "PRIMARY KEY", "FOREIGN KEY", "REFERENCES", "DEFAULT", "SERIAL", "GENERATED ALWAYS AS IDENTITY", "UNIQUE", "CASE", "WHEN", "THEN", "ELSE", "END", "ASC", "DESC", "EXISTS", "CAST", "WITH", "WITH RECURSIVE", "USING"],
    types: ["INTEGER", "BIGINT", "SERIAL", "BIGSERIAL", "SMALLINT", "VARCHAR(255)", "TEXT", "CHAR", "TIMESTAMP", "TIMESTAMPTZ", "DATE", "TIME", "NUMERIC(10,2)", "REAL", "DOUBLE PRECISION", "BOOLEAN", "JSONB", "UUID"],
    funcs: [
      ["COUNT", "COUNT(*) — 统计行数"], ["SUM", "SUM(X)"], ["AVG", "AVG(X)"], ["MIN", "MIN(X)"], ["MAX", "MAX(X)"],
      ["ABS", "ABS(X)"], ["ROUND", "ROUND(X, n)"], ["LENGTH", "LENGTH(X)"], ["LOWER", "LOWER(X)"], ["UPPER", "UPPER(X)"],
      ["TRIM", "TRIM(X)"], ["CONCAT", "CONCAT(A, B, …)"], ["COALESCE", "COALESCE(A, B) — 第一个非 NULL"],
      ["NULLIF", "NULLIF(A, B)"], ["SUBSTRING", "SUBSTRING(X FROM a FOR b)"], ["REPLACE", "REPLACE(X, old, new)"],
      ["POSITION", "POSITION(sub IN X)"], ["NOW", "NOW() — 当前时间戳"], ["CURRENT_DATE", "CURRENT_DATE"],
      ["CURRENT_TIMESTAMP", "CURRENT_TIMESTAMP"], ["TO_CHAR", "TO_CHAR(X, 'YYYY-MM-DD')"], ["DATE_PART", "DATE_PART('year', X)"],
      ["AGE", "AGE(X) — 时间差"], ["STRING_AGG", "STRING_AGG(X, ',') — 聚合拼接"], ["ARRAY_AGG", "ARRAY_AGG(X)"],
      ["JSONB_EXTRACT_PATH", "JSONB_EXTRACT_PATH(X, 'k')"], ["GENERATE_SERIES", "GENERATE_SERIES(1, 10)"],
    ],
    snippets: [
      ["SELECT … FROM … WHERE", "SELECT ${1:*}\nFROM ${2:table}\nWHERE ${3:condition};"],
      ["SELECT … JOIN", "SELECT ${1:*}\nFROM ${2:a}\nJOIN ${3:b} ON ${4:a.id} = ${5:b.a_id}\nWHERE ${6:true};"],
      ["INSERT … RETURNING", "INSERT INTO ${1:table} (${2:col})\nVALUES (${3:value})\nRETURNING *;"],
      ["INSERT … ON CONFLICT", "INSERT INTO ${1:table} (${2:col})\nVALUES (${3:value})\nON CONFLICT (${4:id}) DO NOTHING;"],
      ["UPDATE … SET", "UPDATE ${1:table}\nSET ${2:col} = ${3:value}\nWHERE ${4:condition};"],
      ["DELETE FROM", "DELETE FROM ${1:table}\nWHERE ${2:condition};"],
      ["CREATE TABLE", "CREATE TABLE ${1:name} (\n  id BIGSERIAL PRIMARY KEY,\n  ${2:col} ${3:TEXT}\n);"],
    ],
  },
  redis: {
    label: "Redis",
    commands: [
      ["GET", "GET key — 取字符串值"], ["SET", "SET key value — 设字符串值"], ["DEL", "DEL key — 删除键"],
      ["EXISTS", "EXISTS key — 键是否存在"], ["EXPIRE", "EXPIRE key seconds — 设过期"], ["TTL", "TTL key — 剩余存活秒"],
      ["KEYS", "KEYS pattern — 匹配键（生产慎用）"], ["SCAN", "SCAN cursor — 游标遍历"], ["TYPE", "TYPE key — 键类型"],
      ["INCR", "INCR key — 自增"], ["DECR", "DECR key — 自减"], ["APPEND", "APPEND key value"],
      ["HGET", "HGET key field"], ["HSET", "HSET key field value"], ["HGETALL", "HGETALL key — 取整个哈希"],
      ["HDEL", "HDEL key field"], ["LPUSH", "LPUSH key value"], ["RPUSH", "RPUSH key value"],
      ["LRANGE", "LRANGE key 0 -1 — 取列表"], ["LLEN", "LLEN key — 列表长度"], ["SADD", "SADD key member"],
      ["SMEMBERS", "SMEMBERS key — 取集合"], ["ZADD", "ZADD key score member"], ["ZRANGE", "ZRANGE key 0 -1"],
      ["PING", "PING — 测试连接"], ["INFO", "INFO — 服务器信息"], ["DBSIZE", "DBSIZE — 键数量"],
    ],
  },
};
