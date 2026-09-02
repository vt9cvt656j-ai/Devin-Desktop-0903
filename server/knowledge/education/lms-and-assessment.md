# Education: LMS, Assessment & Adaptive Learning

## LMS Core Architecture

### Course & Enrollment Schema
```sql
CREATE TABLE courses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    description TEXT,
    instructor_id UUID NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'draft',
    -- CHECK (status IN ('draft','published','archived'))
    enrollment_type TEXT DEFAULT 'open',  -- 'open','invite','paid'
    max_students INT,
    starts_at TIMESTAMPTZ,
    ends_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE modules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id UUID NOT NULL REFERENCES courses(id),
    title TEXT NOT NULL,
    position INT NOT NULL,
    unlock_rule TEXT DEFAULT 'sequential',  -- 'sequential','date','manual','always'
    unlock_date TIMESTAMPTZ,
    prerequisite_module_id UUID REFERENCES modules(id)
);

CREATE TABLE lessons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    module_id UUID NOT NULL REFERENCES modules(id),
    title TEXT NOT NULL,
    content_type TEXT NOT NULL,  -- 'video','text','quiz','assignment','interactive'
    content JSONB NOT NULL,
    position INT NOT NULL,
    estimated_minutes INT,
    is_required BOOLEAN DEFAULT true
);

CREATE TABLE enrollments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    course_id UUID NOT NULL REFERENCES courses(id),
    status TEXT NOT NULL DEFAULT 'active',
    -- CHECK (status IN ('active','completed','dropped','expired'))
    enrolled_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    progress_pct SMALLINT DEFAULT 0,
    UNIQUE(user_id, course_id)
);

CREATE TABLE lesson_progress (
    user_id UUID NOT NULL REFERENCES users(id),
    lesson_id UUID NOT NULL REFERENCES lessons(id),
    status TEXT NOT NULL DEFAULT 'not_started',
    -- 'not_started','in_progress','completed'
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    time_spent_sec INT DEFAULT 0,
    PRIMARY KEY (user_id, lesson_id)
);
```

### Progress Tracking
```javascript
async function updateProgress(userId, lessonId) {
  const lesson = await db.getLesson(lessonId);
  const module = await db.getModule(lesson.module_id);
  const courseId = module.course_id;

  await db.upsert('lesson_progress', {
    user_id: userId,
    lesson_id: lessonId,
    status: 'completed',
    completed_at: new Date(),
  });

  // Recalculate course progress
  const totalLessons = await db.count('lessons', { course_id: courseId, is_required: true });
  const completedLessons = await db.query(`
    SELECT COUNT(*) FROM lesson_progress lp
    JOIN lessons l ON l.id = lp.lesson_id
    JOIN modules m ON m.id = l.module_id
    WHERE m.course_id = $1 AND lp.user_id = $2 AND lp.status = 'completed' AND l.is_required = true
  `, [courseId, userId]);

  const pct = Math.round(completedLessons.count / totalLessons * 100);
  await db.query(
    `UPDATE enrollments SET progress_pct = $1, completed_at = CASE WHEN $1 = 100 THEN NOW() ELSE NULL END
     WHERE user_id = $2 AND course_id = $3`,
    [pct, userId, courseId]
  );
}
```

## Quiz & Assessment

### Question Data Model
```sql
CREATE TABLE quizzes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    lesson_id UUID NOT NULL REFERENCES lessons(id),
    title TEXT NOT NULL,
    time_limit_sec INT,
    max_attempts INT DEFAULT 1,
    passing_score SMALLINT DEFAULT 70,   -- percentage
    shuffle_questions BOOLEAN DEFAULT false,
    show_answers_after TEXT DEFAULT 'submission', -- 'never','submission','deadline'
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE questions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    quiz_id UUID NOT NULL REFERENCES quizzes(id),
    question_type TEXT NOT NULL,
    -- 'multiple_choice','multi_select','true_false','short_answer','fill_blank',
    -- 'matching','ordering','essay','code','parsons'
    body TEXT NOT NULL,       -- question text (supports markdown + LaTeX)
    options JSONB,            -- for MC: [{id, text, is_correct}]
    correct_answer JSONB,     -- type-specific: string, array, object
    points INT NOT NULL DEFAULT 1,
    explanation TEXT,         -- shown after grading
    position INT NOT NULL
);

-- Answer storage
CREATE TABLE quiz_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    quiz_id UUID NOT NULL,
    user_id UUID NOT NULL,
    attempt_number INT NOT NULL,
    answers JSONB NOT NULL,       -- {question_id: user_answer}
    score SMALLINT,               -- percentage
    points_earned INT,
    points_possible INT,
    started_at TIMESTAMPTZ,
    submitted_at TIMESTAMPTZ,
    graded_at TIMESTAMPTZ
);
```

### Auto-Grading Engine
```python
class Grader:
    def grade_question(self, question, user_answer):
        if question.question_type == 'multiple_choice':
            return 1 if user_answer == question.correct_answer else 0

        elif question.question_type == 'multi_select':
            correct = set(question.correct_answer)
            selected = set(user_answer)
            if selected == correct:
                return 1
            # Partial credit: (correct_selected - incorrect_selected) / total_correct
            correct_selected = len(selected & correct)
            incorrect_selected = len(selected - correct)
            return max(0, (correct_selected - incorrect_selected) / len(correct))

        elif question.question_type == 'fill_blank':
            acceptable = [a.lower().strip() for a in question.correct_answer]
            return 1 if user_answer.lower().strip() in acceptable else 0

        elif question.question_type == 'ordering':
            return 1 if user_answer == question.correct_answer else 0

        elif question.question_type == 'matching':
            correct = question.correct_answer  # {left_id: right_id}
            matches = sum(1 for k, v in user_answer.items() if correct.get(k) == v)
            return matches / len(correct)

        elif question.question_type == 'code':
            return self._grade_code(question, user_answer)

        elif question.question_type == 'parsons':
            return self._grade_parsons(question, user_answer)

        elif question.question_type == 'essay':
            return None  # manual grading required

    def _grade_code(self, question, code):
        """Run user code against test cases in sandbox"""
        result = sandbox.execute(
            code=code,
            language=question.correct_answer['language'],
            test_cases=question.correct_answer['tests'],
            timeout_sec=10,
            memory_limit_mb=128,
        )
        passed = sum(1 for t in result.tests if t.passed)
        return passed / len(result.tests)

    def _grade_parsons(self, question, user_order):
        """Parsons problem: rearrange code lines into correct order"""
        correct = question.correct_answer['line_order']
        distractors = set(question.correct_answer.get('distractors', []))
        # Penalize: included distractors or wrong order
        user_filtered = [l for l in user_order if l not in distractors]
        if user_filtered == correct:
            return 1
        # Partial: longest common subsequence / total
        lcs = self._lcs_length(user_filtered, correct)
        return lcs / len(correct)
```

## Spaced Repetition

### FSRS Algorithm (Free Spaced Repetition Scheduler)
```python
import math
from dataclasses import dataclass
from datetime import datetime, timedelta

@dataclass
class Card:
    difficulty: float = 0.3    # D ∈ [0, 1]
    stability: float = 1.0     # S (days until R drops to 90%)
    due_date: datetime = None
    reps: int = 0
    lapses: int = 0

# FSRS-5 parameters (pre-trained, can be personalized)
W = [0.4072, 1.1829, 3.1262, 15.4722, 7.2102, 0.5316, 1.0651, 0.0589,
     1.5747, 0.1070, 1.0621, 1.9395, 0.1100, 0.2905, 2.2698, 0.2315, 2.9898, 0.5200, 0.6590]

def fsrs_schedule(card, rating):
    """
    rating: 1=Again, 2=Hard, 3=Good, 4=Easy
    Returns updated card with new due_date
    """
    if card.reps == 0:
        # First review: initial stability from W[0..3]
        card.stability = W[rating - 1]
        card.difficulty = W[4] - math.exp(W[5] * (rating - 1)) + 1
    else:
        elapsed = (datetime.now() - card.due_date).days if card.due_date else 1
        retrievability = math.exp(math.log(0.9) * elapsed / card.stability)

        if rating == 1:  # lapse
            card.lapses += 1
            card.stability = W[11] * card.difficulty ** (-W[12]) * (
                (card.stability + 1) ** W[13] - 1) * math.exp(W[14] * (1 - retrievability))
        else:
            card.stability = card.stability * (1 + math.exp(W[8]) * (11 - card.difficulty) *
                card.stability ** (-W[9]) * (math.exp(W[10] * (1 - retrievability)) - 1) *
                (W[15] if rating == 2 else 1) * (W[16] if rating == 4 else 1))

        card.difficulty = W[7] * (card.difficulty - W[6]) + W[6]

    card.difficulty = max(0.01, min(0.99, card.difficulty))
    card.stability = max(0.1, card.stability)
    interval = max(1, round(card.stability * 9))  # R=90% target
    card.due_date = datetime.now() + timedelta(days=interval)
    card.reps += 1
    return card
```

### SM-2 Algorithm (Classic Anki)
```python
def sm2_schedule(card, quality):
    """
    quality: 0-5 (0-2 = fail, 3 = hard, 4 = good, 5 = easy)
    """
    if quality < 3:
        card.reps = 0
        card.interval = 1
    else:
        if card.reps == 0:
            card.interval = 1
        elif card.reps == 1:
            card.interval = 6
        else:
            card.interval = round(card.interval * card.ease_factor)

        card.ease_factor = max(1.3,
            card.ease_factor + 0.1 - (5 - quality) * (0.08 + (5 - quality) * 0.02))
        card.reps += 1

    card.due_date = datetime.now() + timedelta(days=card.interval)
    return card
```

## LTI Integration (Learning Tools Interoperability)

### LTI 1.3 Launch Flow
```
1. Platform (LMS) → Tool: OIDC login initiation
   POST /lti/login { iss, login_hint, target_link_uri, lti_message_hint }

2. Tool → Platform: Authentication request
   Redirect to platform's auth endpoint with state + nonce

3. Platform → Tool: ID Token (JWT)
   POST /lti/launch { id_token }
   Claims: sub, name, email, roles, context(course), resource_link, custom params

4. Tool validates JWT signature (platform's public key from JWKS endpoint)
5. Tool establishes session, renders content
```

### LTI Advantage Services
```javascript
// Assignment and Grade Services (AGS) — send grades back to LMS
async function submitGrade(launchData, userId, score) {
  const token = await getLtiAccessToken(launchData, [
    'https://purl.imsglobal.org/spec/lti-ags/scope/score',
  ]);

  await fetch(launchData.lineitem_url + '/scores', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/vnd.ims.lis.v1.score+json',
    },
    body: JSON.stringify({
      userId: launchData.sub,
      scoreGiven: score,
      scoreMaximum: 100,
      activityProgress: 'Completed',
      gradingProgress: 'FullyGraded',
      timestamp: new Date().toISOString(),
    }),
  });
}

// Names and Role Provisioning Services (NRPS) — get roster
async function getRoster(launchData) {
  const token = await getLtiAccessToken(launchData, [
    'https://purl.imsglobal.org/spec/lti-nrps/scope/contextmembership.readonly',
  ]);
  const res = await fetch(launchData.memberships_url, {
    headers: { 'Authorization': `Bearer ${token}` },
  });
  return res.json(); // { members: [{user_id, name, email, roles}] }
}
```

## Interactive Exercise Patterns

### Fill-in-the-Blank (Code)
```javascript
function renderCodeFillBlank(template, blanks) {
  // template: "function add(a, b) {\n  return ___BLANK_1___;\n}"
  // blanks: [{id: 'BLANK_1', answer: 'a + b', hint: 'arithmetic operator'}]

  let html = escapeHtml(template);
  for (const blank of blanks) {
    const placeholder = `___${blank.id}___`;
    const input = `<input type="text" data-blank="${blank.id}"
      class="code-blank" autocomplete="off"
      placeholder="${blank.hint || '...'}" />`;
    html = html.replace(placeholder, input);
  }
  return `<pre class="code-exercise">${html}</pre>`;
}

function gradeCodeBlanks(blanks, userAnswers) {
  return blanks.map(b => {
    const answer = (userAnswers[b.id] || '').trim();
    const acceptable = [b.answer, ...(b.alternatives || [])].map(a => a.trim());
    return { id: b.id, correct: acceptable.includes(answer) };
  });
}
```

### Live Code Execution (Sandboxed)
```javascript
async function executeInSandbox(code, language, testCases, limits = {}) {
  const config = {
    timeout_ms: limits.timeout || 10000,
    memory_mb: limits.memory || 128,
    network: false,
    filesystem: 'readonly',
  };

  // Docker-based sandbox or WASM (Pyodide for Python, QuickJS for JS)
  const result = await sandbox.run({ code, language, config });

  const testResults = [];
  for (const tc of testCases) {
    const output = await sandbox.run({
      code: `${code}\n${tc.test_code}`,
      language,
      config,
    });
    testResults.push({
      name: tc.name,
      passed: output.stdout.trim() === tc.expected_output.trim(),
      actual: output.stdout,
      expected: tc.expected_output,
      error: output.stderr || null,
    });
  }

  return { output: result, tests: testResults };
}
```

### Parsons Problems (Drag-and-Drop Code Ordering)
```javascript
function createParsonsExercise(config) {
  // config.lines: correct code lines in order
  // config.distractors: wrong lines that shouldn't be used
  // config.indentation: whether indentation matters

  const allLines = [...config.lines, ...config.distractors];
  const shuffled = shuffleArray(allLines);

  return {
    type: 'parsons',
    lines: shuffled.map((line, i) => ({
      id: `line_${i}`,
      text: config.indentation ? line.trimStart() : line,
      isDistractor: config.distractors.includes(line),
    })),
    correct_order: config.lines.map(l => shuffled.indexOf(l)),
    indentation_matters: config.indentation,
  };
}
```

## Plagiarism Detection

### Approach: Structural Similarity (Code)
```python
import ast

def code_similarity(code_a, code_b):
    """AST-based structural comparison (language-agnostic with tree-sitter)"""
    tree_a = ast.parse(code_a)
    tree_b = ast.parse(code_b)

    # Normalize: strip comments, rename variables to generic names
    norm_a = normalize_ast(tree_a)
    norm_b = normalize_ast(tree_b)

    # Winnowing algorithm (robust fingerprinting)
    fp_a = set(winnow(hash_ngrams(ast_to_tokens(norm_a), n=5), window=4))
    fp_b = set(winnow(hash_ngrams(ast_to_tokens(norm_b), n=5), window=4))

    if not fp_a or not fp_b:
        return 0.0
    intersection = fp_a & fp_b
    return len(intersection) / min(len(fp_a), len(fp_b))

def normalize_ast(tree):
    """Rename all variables to V1, V2, ... — defeats simple renaming"""
    var_map = {}
    counter = [0]
    for node in ast.walk(tree):
        if isinstance(node, ast.Name):
            if node.id not in var_map:
                var_map[node.id] = f'V{counter[0]}'
                counter[0] += 1
            node.id = var_map[node.id]
    return tree
```

### Text Plagiarism (Essay/Report)
```python
def text_similarity(text_a, text_b, n=5):
    """Jaccard similarity on character n-grams (fast, baseline)"""
    ngrams_a = set(text_a[i:i+n] for i in range(len(text_a) - n + 1))
    ngrams_b = set(text_b[i:i+n] for i in range(len(text_b) - n + 1))
    if not ngrams_a or not ngrams_b:
        return 0.0
    return len(ngrams_a & ngrams_b) / len(ngrams_a | ngrams_b)

# Thresholds:
# > 0.8: almost certainly copied
# 0.5-0.8: suspicious, needs manual review
# < 0.5: likely original (or heavily paraphrased)
```

## Student Analytics

### Learning Analytics Schema
```sql
CREATE TABLE learning_events (
    id BIGSERIAL,
    user_id UUID NOT NULL,
    course_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    -- 'video_play','video_pause','video_seek','page_view','quiz_start',
    -- 'quiz_submit','assignment_submit','forum_post','resource_download'
    resource_id UUID,
    metadata JSONB,       -- event-specific: {video_position, time_spent, score}
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- Risk indicators (at-risk student detection)
CREATE VIEW student_risk AS
SELECT
    e.user_id,
    e.course_id,
    MAX(e.created_at) AS last_activity,
    NOW() - MAX(e.created_at) AS inactive_days,
    AVG(CASE WHEN qa.score IS NOT NULL THEN qa.score END) AS avg_quiz_score,
    en.progress_pct
FROM enrollments en
JOIN learning_events e ON e.user_id = en.user_id AND e.course_id = en.course_id
LEFT JOIN quiz_attempts qa ON qa.user_id = en.user_id
WHERE en.status = 'active'
GROUP BY e.user_id, e.course_id, en.progress_pct
HAVING NOW() - MAX(e.created_at) > interval '7 days'
   OR AVG(qa.score) < 50;
```

## Common LLM Mistakes in EdTech
```
1. Not randomizing question order and answer options (enables answer sharing)
2. Storing quiz answers in client-side JavaScript (inspect → cheat)
3. Using client-side timers for timed quizzes (manipulable)
4. Not normalizing code submissions before comparison (whitespace, variable names)
5. Grading essays with LLM without human oversight flag
6. Missing accessibility in interactive exercises (screen reader, keyboard nav)
7. Not tracking time-on-task per question (analytics gold, often omitted)
8. Sending grades without LTI signature verification
9. Hardcoding FSRS/SM-2 parameters instead of making them tunable
10. Not handling timezone in course deadlines (store UTC, display local)
```
