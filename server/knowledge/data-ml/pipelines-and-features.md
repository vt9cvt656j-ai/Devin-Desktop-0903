# Data Pipelines, Feature Engineering & ML Training

## Data Pipeline Design

### Pipeline Generation (Prompt2DAG: 78.5% success)
```
1. Decompose pipeline request into structured workflow spec (steps, dependencies, data sources)
2. Generate workflow structure as JSON/YAML first
3. Translate each node to executable code
4. NEVER generate the full DAG in one shot
5. Validate: python -c "import dag_file" + airflow dags test
```

### Data Validation (Automated — saves 8+ hours/table)
```
For each table, generate validation rules covering:
- Null checks per column
- Type validation
- Range constraints (min/max for numerics)
- Uniqueness constraints
- Referential integrity (FK relationships)
- Distribution checks (standard deviation, percentiles)

Rule: assertions MUST reflect intended behavior, NOT observed data patterns
(avoid locking in bugs as "expected")
```

### Spark Best Practices
- Partition by frequently filtered columns, ~128MB per partition
- Broadcast joins for small tables (< 100MB)
- Cache intermediate DataFrames used in multiple paths
- Never `.collect()` on large datasets
- Check explain plan for unnecessary shuffles
- Never generate DML (INSERT/UPDATE/DELETE/DROP) unless explicitly requested

## Feature Engineering

### Iterative Feature Generation (CAAFE: ROC AUC 0.798→0.822)
```
1. Read all column names, types, and task description
2. Propose 3-5 new features based on semantic understanding of columns
3. Generate feature code → evaluate via cross-validation → keep if metric improves
4. Include domain explanation alongside each feature
5. Maintain "feature memory" — top-K performing transforms as few-shot examples
6. Use 3 independent generation threads to avoid local optima
7. Do NOT include raw numeric data samples (LLMs struggle with numeric comprehension)
```

### Feature Sanity Checks
Before adding any feature, verify:
- Does it leak target information?
- Is it computable at inference time?
- Does it introduce data from the future?
- Is it redundant with existing features? (correlation > 0.95 → drop)
- After adding: run XGBoost feature importance to validate usefulness

## Model Training

### Hyperparameter Tuning (Structured TCS: within 0.9pp of GPT-4)
```
After each training trial, produce a structured summary:
1. Current optimization status (converging/stagnating/diverging)
2. Performance gap from target
3. Latest experiment results with specific hyperparameters
4. Per-parameter historical analysis
5. Comparative impact of recent changes

Feed this summary (not raw logs) to next iteration.
Change only the single most impactful parameter per iteration.
```

### Experiment Tracking
```python
# Every training script includes:
import mlflow
mlflow.autolog()

# Or explicit tracking:
with mlflow.start_run():
    mlflow.log_params({"lr": 0.001, "epochs": 50})
    mlflow.log_metrics({"val_acc": 0.92, "val_loss": 0.31})
    mlflow.log_artifact("model.pkl")
```

### Fine-Tuning Decision Tree
```
Start with prompt engineering
→ If insufficient, add RAG
→ If still insufficient, fine-tune with SFT (1000-5000 examples)
→ For alignment, use ORPO (not DPO/RLHF — eliminates reference model)
→ QLoRA defaults: rank=16, alpha=32, target all linear layers, BF16, 4-bit NF4
→ Quality >>> quantity for training data
```

## Data Visualization

### Chart Selection Matrix
| Data Pattern | Chart Type |
|-------------|------------|
| Categorical comparison | Bar chart |
| Time series | Line chart |
| Distribution | Histogram / KDE |
| Correlation | Scatter / Heatmap |
| Part-of-whole | Pie / Treemap |
| Geospatial | Choropleth |

### Dashboard Design
```
1. Analyze through three lenses: descriptive, predictive, domain-specific
2. Generate dashboard specs as structured JSON (charts, layout, filters)
3. Map to rendering library (Plotly/Streamlit/Grafana) in second pass
4. Always include insight annotations (title + subtitle explaining finding)
5. Two-pass approach cuts token cost and improves visual consistency
```

## SQL Optimization

### Query Generation (AgentNLQ: 60.2%→78.1% accuracy)
```
1. Pre-process schemas: generate descriptions for every table and column
2. On query failure, compress error into structured summary:
   - Original question
   - Prior attempts
   - Error messages
   - Avoidance instructions
3. Validate via SQLGlot (syntax) AND execution (semantics)
4. Efficiency: check EXPLAIN plan, suggest indexes, prefer window functions
```

## RAG Architecture

### Default Recipe (recall +26%, precision +28%)
```
Retrieval: hybrid (dense + BM25 + RRF at k=60)
Reranking: cross-encoder (ColBERT v2 or Cohere Rerank 3.5)
Chunking: semantic at 300-500 tokens with 10-20% overlap
Query: Multi-Query + RRF (3-5 variants, retrieve independently, fuse)

80% of RAG problems are retrieval problems, not generation problems.
Debug retrieval first.
```

### RAG Evaluation Targets (RAGAS)
- Faithfulness > 0.8
- Answer Relevancy > 0.8
- Context Precision > 0.8
- Context Recall > 0.7
