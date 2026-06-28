import { useEffect, useState } from "react"
import { Separator } from "@/components/ui/separator"

interface Summary {
  total: number
  blocked: number
  total_today: number
  blocked_today: number
}

interface DomainCount {
  domain: string
  count: number
}

export function Analytics() {
  const [summary, setSummary] = useState<Summary | null>(null)
  const [topBlocked, setTopBlocked] = useState<DomainCount[]>([])
  const [topQueried, setTopQueried] = useState<DomainCount[]>([])

  useEffect(() => {
    fetch("/analytics")
      .then((r) => r.json())
      .then(setSummary)
      .catch(() => {})
    fetch("/analytics/top_blocked")
      .then((r) => r.json())
      .then((d) => setTopBlocked(d.domains ?? []))
      .catch(() => {})
    fetch("/analytics/top_queried")
      .then((r) => r.json())
      .then((d) => setTopQueried(d.domains ?? []))
      .catch(() => {})
  }, [])

  const blockRate =
    summary && summary.total > 0
      ? ((summary.blocked / summary.total) * 100).toFixed(1)
      : "0.0"

  const todayRate =
    summary && summary.total_today > 0
      ? ((summary.blocked_today / summary.total_today) * 100).toFixed(1)
      : "0.0"

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <StatCard label="Total Queries" value={summary?.total ?? null} />
        <StatCard label="Total Blocked" value={summary?.blocked ?? null} sub={`${blockRate}% block rate`} />
        <StatCard label="Queries Today" value={summary?.total_today ?? null} />
        <StatCard label="Blocked Today" value={summary?.blocked_today ?? null} sub={`${todayRate}% block rate`} />
      </div>

      <div className="flex gap-4">
        <DomainList title="Top Blocked" domains={topBlocked} />
        <DomainList title="Top Queried" domains={topQueried} />
      </div>
    </div>
  )
}

function StatCard({ label, value, sub }: { label: string; value: number | null; sub?: string }) {
  return (
    <div className="rounded-md border p-4">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-2xl font-semibold tabular-nums">
        {value === null ? <span className="text-muted-foreground/40">—</span> : value.toLocaleString()}
      </p>
      {sub && <p className="mt-0.5 text-xs text-muted-foreground">{sub}</p>}
    </div>
  )
}

function DomainList({ title, domains }: { title: string; domains: DomainCount[] }) {
  return (
    <div className="flex flex-1 flex-col gap-2">
      <p className="text-sm font-medium">{title}</p>
      <div className="rounded-md border">
        {domains.length === 0 ? (
          <p className="px-3 py-3 text-center text-xs text-muted-foreground">No data yet.</p>
        ) : (
          domains.map((d, i) => (
            <div key={d.domain}>
              {i > 0 && <Separator />}
              <div className="flex items-center gap-2 px-3 py-2">
                <span className="flex-1 truncate text-xs" title={d.domain}>
                  {d.domain}
                </span>
                <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                  {d.count.toLocaleString()}
                </span>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
