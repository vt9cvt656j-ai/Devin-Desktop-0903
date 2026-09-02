import { H1, Stack, Text, useHostTheme } from 'qoder/canvas';

/**
 * Google White Card 任务列表 Demo
 * 
 * 设计规范：
 * - 纯白底 #FFFFFF
 * - 圆角 12px
 * - 微阴影 0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06)
 * - 无边框（或极浅 #F0F0F0）
 * - 内边距 16px 20px
 * - 文字 #1F1F1F（主）/ #5F6368（次）
 * - 字体 Google Sans / system-ui
 * - 卡片间距 12px
 * - hover 时阴影加深：0 4px 12px rgba(0,0,0,0.1)
 */

const tasks = [
  { id: 1, title: '改 README 示例代码，与源码 API 对齐', tag: '文档', done: false },
  { id: 2, title: 'Demo 里对 search_videos / get_user_videos 加 try/except 优雅捕获', tag: '代码', done: false },
  { id: 3, title: 'README 里要么删掉"视频详情/评论"的功能声明，要么补上"需要 Cookie"的说明', tag: '文档', done: false },
  { id: 4, title: '加 .gitignore', tag: '工程', done: true },
];

const tagColors: Record<string, string> = {
  '文档': '#E8F0FE',
  '代码': '#E6F4EA',
  '工程': '#FEF7E0',
};

const tagTextColors: Record<string, string> = {
  '文档': '#1967D2',
  '代码': '#137333',
  '工程': '#B06000',
};

function GoogleWhiteCard({ task }: { task: typeof tasks[0] }) {
  return (
    <div style={{
      background: '#FFFFFF',
      borderRadius: '12px',
      padding: '16px 20px',
      boxShadow: '0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06)',
      display: 'flex',
      alignItems: 'flex-start',
      gap: '12px',
      transition: 'box-shadow 0.2s ease',
      cursor: 'default',
    }}
    onMouseEnter={(e) => {
      (e.currentTarget as HTMLDivElement).style.boxShadow = '0 4px 12px rgba(0,0,0,0.12)';
    }}
    onMouseLeave={(e) => {
      (e.currentTarget as HTMLDivElement).style.boxShadow = '0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06)';
    }}
    >
      {/* Checkbox */}
      <div style={{
        width: '20px',
        height: '20px',
        borderRadius: '50%',
        border: task.done ? 'none' : '2px solid #DADCE0',
        background: task.done ? '#1A73E8' : 'transparent',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexShrink: 0,
        marginTop: '2px',
      }}>
        {task.done && (
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M2 6L5 9L10 3" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        )}
      </div>

      {/* Content */}
      <div style={{ flex: 1 }}>
        <div style={{
          fontSize: '14px',
          lineHeight: '1.5',
          color: task.done ? '#9AA0A6' : '#202124',
          textDecoration: task.done ? 'line-through' : 'none',
          fontFamily: '"Google Sans", "Noto Sans SC", system-ui, sans-serif',
        }}>
          <span style={{ color: '#5F6368', marginRight: '4px' }}>{task.id}、</span>
          {task.title}
        </div>
      </div>

      {/* Tag */}
      <span style={{
        fontSize: '11px',
        fontWeight: 500,
        padding: '2px 8px',
        borderRadius: '4px',
        background: tagColors[task.tag] || '#F1F3F4',
        color: tagTextColors[task.tag] || '#5F6368',
        flexShrink: 0,
        fontFamily: '"Google Sans", system-ui, sans-serif',
      }}>
        {task.tag}
      </span>
    </div>
  );
}

export default function GoogleWhiteCardDemo() {
  const theme = useHostTheme();

  return (
    <div style={{
      padding: '32px',
      background: '#F8F9FA',
      minHeight: '100%',
      fontFamily: '"Google Sans", "Noto Sans SC", system-ui, sans-serif',
    }}>
      <Stack gap={24}>
        {/* Header */}
        <div>
          <H1>任务卡片 · Google White Style</H1>
          <Text tone="secondary" size="small" style={{ marginTop: '4px' }}>
            Material Design 白卡规范：纯白底 / 12px 圆角 / 微阴影 / 无边框 / hover 加深
          </Text>
        </div>

        {/* Task Cards */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', maxWidth: '640px' }}>
          {tasks.map((task) => (
            <GoogleWhiteCard key={task.id} task={task} />
          ))}
        </div>

        {/* Spec Documentation */}
        <div style={{
          background: '#FFFFFF',
          borderRadius: '12px',
          padding: '20px 24px',
          boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
          maxWidth: '640px',
        }}>
          <div style={{ fontSize: '13px', fontWeight: 600, color: '#202124', marginBottom: '12px' }}>
            设计规范速查
          </div>
          <table style={{ width: '100%', fontSize: '12px', color: '#5F6368', borderCollapse: 'collapse' }}>
            <tbody>
              {[
                ['背景', '#FFFFFF 纯白'],
                ['圆角', '12px'],
                ['阴影', '0 1px 3px rgba(0,0,0,0.08)'],
                ['Hover 阴影', '0 4px 12px rgba(0,0,0,0.12)'],
                ['内边距', '16px 20px'],
                ['卡片间距', '12px'],
                ['主文字', '#202124 / 14px'],
                ['次文字', '#5F6368'],
                ['字体', 'Google Sans, Noto Sans SC, system-ui'],
                ['Checkbox', '20px 圆形 / 选中 #1A73E8'],
                ['Tag 圆角', '4px'],
              ].map(([k, v]) => (
                <tr key={k} style={{ borderBottom: '1px solid #F1F3F4' }}>
                  <td style={{ padding: '6px 0', fontWeight: 500, color: '#202124', width: '100px' }}>{k}</td>
                  <td style={{ padding: '6px 0' }}>{v}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Stack>
    </div>
  );
}
