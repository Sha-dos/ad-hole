import { Field, FieldLabel } from "@/components/ui/field.tsx"
import { Input } from "@/components/ui/input.tsx"
import { Button } from "@/components/ui/button.tsx"
import { Switch } from "@/components/ui/switch.tsx"
import { Separator } from "@/components/ui/separator.tsx"
import { Dialog, DialogContent, DialogHeader, DialogFooter, DialogTitle } from "@/components/ui/dialog.tsx"
import { Loader2, Trash2 } from "lucide-react"
import { toast } from "sonner"
import { useState, useEffect } from "react"

interface Source {
  url: string
  enabled: boolean
}

interface Overrides {
  added: string[]
  removed: string[]
}

type Action = "block" | "unblock" | "source-add" | "source-remove" | "override-unblock" | "override-unremove"

interface Pending {
  action: Action
  value: string
}

const TITLES: Record<Action, string> = {
  block: "Confirm block",
  unblock: "Confirm unblock",
  "source-add": "Add source",
  "source-remove": "Remove source",
  "override-unblock": "Remove override",
  "override-unremove": "Remove override",
}

function ConfirmBody({ pending }: { pending: Pending }) {
  const val = <span className="font-medium text-foreground break-all">{pending.value}</span>
  if (pending.action === "block") return <>Block {val}?</>
  if (pending.action === "unblock") return <>Unblock {val}?</>
  if (pending.action === "source-add") return <>Add {val} as a blocklist source?</>
  if (pending.action === "source-remove") return <>Remove {val}? This source will no longer be fetched.</>
  if (pending.action === "override-unblock") return <>Remove manual block on {val}? It may still be blocked by a source list.</>
  return <>Remove manual unblock on {val}? It may be re-blocked by a source list.</>
}

export function App() {
  const [sources, setSources] = useState<Source[]>([])
  const [overrides, setOverrides] = useState<Overrides>({ added: [], removed: [] })
  const [newSourceUrl, setNewSourceUrl] = useState("")
  const [blockDomain, setBlockDomain] = useState("")
  const [unblockDomain, setUnblockDomain] = useState("")
  const [pending, setPending] = useState<Pending | null>(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    fetch("/sources")
      .then((r) => r.json())
      .then((d) => setSources(d.sources ?? []))
      .catch(() => {})
    fetch("/overrides")
      .then((r) => r.json())
      .then((d) => setOverrides({ added: d.added ?? [], removed: d.removed ?? [] }))
      .catch(() => {})
  }, [])

  const openConfirm = (action: Action, value: string) => {
    if (!value) return
    setPending({ action, value })
  }

  const handleToggle = async (url: string, enabled: boolean) => {
    setSources((prev) => prev.map((s) => (s.url === url ? { ...s, enabled } : s)))
    try {
      const res = await fetch("/sources", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url, action: "toggle", enabled }),
      })
      if (res.ok) {
        toast.success(enabled ? "Source enabled" : "Source disabled")
      } else {
        setSources((prev) => prev.map((s) => (s.url === url ? { ...s, enabled: !enabled } : s)))
        toast.error("Failed to toggle source")
      }
    } catch {
      setSources((prev) => prev.map((s) => (s.url === url ? { ...s, enabled: !enabled } : s)))
      toast.error("Failed to toggle source")
    }
  }

  const handleConfirm = async () => {
    if (!pending) return
    setLoading(true)
    try {
      let res: Response

      if (pending.action === "block") {
        res = await fetch("/update_blocklist", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action: "add", domain: pending.value }),
        })
        if (res.ok) {
          setBlockDomain("")
          setOverrides((o) => ({ ...o, added: [...o.added, pending.value].sort() }))
          toast.success(`${pending.value} blocked`)
        }
      } else if (pending.action === "unblock") {
        res = await fetch("/update_blocklist", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action: "remove", domain: pending.value }),
        })
        if (res.ok) {
          setUnblockDomain("")
          setOverrides((o) => ({ ...o, removed: [...o.removed, pending.value].sort() }))
          toast.success(`${pending.value} unblocked`)
        }
      } else if (pending.action === "source-add") {
        res = await fetch("/sources", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url: pending.value, action: "add" }),
        })
        if (res.ok) {
          setSources((prev) => [...prev, { url: pending.value, enabled: true }])
          setNewSourceUrl("")
          toast.success("Source added")
        }
      } else if (pending.action === "source-remove") {
        res = await fetch("/sources", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url: pending.value, action: "remove" }),
        })
        if (res.ok) {
          setSources((prev) => prev.filter((s) => s.url !== pending.value))
          toast.success("Source removed")
        }
      } else if (pending.action === "override-unblock") {
        res = await fetch("/update_blocklist", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action: "remove", domain: pending.value }),
        })
        if (res.ok) {
          setOverrides((o) => ({ ...o, added: o.added.filter((d) => d !== pending.value) }))
          toast.success(`Manual block on ${pending.value} removed`)
        }
      } else {
        res = await fetch("/update_blocklist", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action: "add", domain: pending.value }),
        })
        if (res.ok) {
          setOverrides((o) => ({ ...o, removed: o.removed.filter((d) => d !== pending.value) }))
          toast.success(`Manual unblock on ${pending.value} removed`)
        }
      }

      if (res!.ok) {
        setPending(null)
      } else {
        toast.error("Something went wrong")
      }
    } catch {
      toast.error("Something went wrong")
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-svh p-6 pt-12">
      <div className="flex gap-4">
        <div className="flex min-w-0 flex-1 flex-col gap-3">
          <Field>
            <FieldLabel>Add source</FieldLabel>
            <div className="flex gap-2">
              <Input
                value={newSourceUrl}
                onChange={(e) => setNewSourceUrl(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && openConfirm("source-add", newSourceUrl.trim())}
                placeholder="https://example.com/list.txt"
              />
              <Button variant="outline" onClick={() => openConfirm("source-add", newSourceUrl.trim())}>
                Add
              </Button>
            </div>
          </Field>
          <div className="rounded-md border">
            {sources.length === 0 && (
              <p className="px-3 py-3 text-center text-xs text-muted-foreground">No sources.</p>
            )}
            {sources.map((src, i) => (
              <div key={src.url}>
                {i > 0 && <Separator />}
                <div className="flex items-center gap-3 px-3 py-2">
                  <Switch
                    checked={src.enabled}
                    onCheckedChange={(checked) => handleToggle(src.url, checked)}
                  />
                  <span className="flex-1 truncate text-xs text-muted-foreground" title={src.url}>
                    {src.url}
                  </span>
                  <Button size="icon-sm" variant="ghost" onClick={() => openConfirm("source-remove", src.url)}>
                    <Trash2 />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="flex w-52 shrink-0 flex-col gap-3">
          <Field>
            <FieldLabel>Block domain</FieldLabel>
            <div className="flex gap-2">
              <Input
                value={blockDomain}
                onChange={(e) => setBlockDomain(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && openConfirm("block", blockDomain.trim())}
                placeholder="example.com"
              />
              <Button onClick={() => openConfirm("block", blockDomain.trim())}>Block</Button>
            </div>
          </Field>
          <div className="rounded-md border">
            {overrides.added.length === 0 && (
              <p className="px-3 py-3 text-center text-xs text-muted-foreground">None.</p>
            )}
            {overrides.added.map((domain, i) => (
              <div key={domain}>
                {i > 0 && <Separator />}
                <div className="flex items-center gap-2 px-3 py-2">
                  <span className="flex-1 truncate text-xs" title={domain}>{domain}</span>
                  <Button size="icon-sm" variant="ghost" onClick={() => openConfirm("override-unblock", domain)}>
                    <Trash2 />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="flex w-52 shrink-0 flex-col gap-3">
          <Field>
            <FieldLabel>Unblock domain</FieldLabel>
            <div className="flex gap-2">
              <Input
                value={unblockDomain}
                onChange={(e) => setUnblockDomain(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && openConfirm("unblock", unblockDomain.trim())}
                placeholder="example.com"
              />
              <Button variant="destructive" onClick={() => openConfirm("unblock", unblockDomain.trim())}>Unblock</Button>
            </div>
          </Field>
          <div className="rounded-md border">
            {overrides.removed.length === 0 && (
              <p className="px-3 py-3 text-center text-xs text-muted-foreground">None.</p>
            )}
            {overrides.removed.map((domain, i) => (
              <div key={domain}>
                {i > 0 && <Separator />}
                <div className="flex items-center gap-2 px-3 py-2">
                  <span className="flex-1 truncate text-xs text-muted-foreground" title={domain}>{domain}</span>
                  <Button size="icon-sm" variant="ghost" onClick={() => openConfirm("override-unremove", domain)}>
                    <Trash2 />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </div>

      </div>

      <Dialog open={pending !== null} onOpenChange={(open) => !open && !loading && setPending(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{pending && TITLES[pending.action]}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            {pending && <ConfirmBody pending={pending} />}
          </p>
          <DialogFooter>
            <Button variant="outline" disabled={loading} onClick={() => setPending(null)}>
              Cancel
            </Button>
            <Button
              variant={
                pending?.action === "source-remove" ||
                pending?.action === "unblock" ||
                pending?.action === "override-unblock" ||
                pending?.action === "override-unremove"
                  ? "destructive"
                  : "default"
              }
              disabled={loading}
              onClick={handleConfirm}
            >
              {loading ? <Loader2 className="size-3.5 animate-spin" /> : "Confirm"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default App
