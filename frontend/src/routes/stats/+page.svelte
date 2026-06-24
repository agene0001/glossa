<div class="page-head">
	<h1>Stats</h1>
	<p>Mastery trends over time.</p>
</div>

<div class="card">
	<p>
		Trend analytics (exposure → mastery latency, words-per-session, frequency-priority queues)
		come from the <strong>DuckDB</strong> analytics read path described in spec §5/§6. V1 records
		every event append-only, so the history needed to chart this is already being captured — the
		charts are a read-side feature, not new instrumentation.
	</p>
	<p class="muted">For now, the <a href="/review">Review</a> tab shows your current graph state.</p>
</div>
