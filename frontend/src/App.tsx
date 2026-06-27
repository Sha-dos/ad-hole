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

type Action = "block" | "unblock" | "source-add" | "source-remove"

interface Pending {
  action: Action
  value: string
}

const TITLES: Record<Action, string> = {
  block: "Confirm block",
  unblock: "Confirm unblock",
  "source-add": "Add source",
  "source-remove": "Remove source",
}

function ConfirmBody({ pending }: { pending: Pending }) {
  const val = <span className="font-medium text-foreground break-all">{pending.value}</span>
  if (pending.action === "block") return <>Block {val}?</>
  if (pending.action === "unblock") return <>Unblock {val}?</>
  if (pending.action === "source-add") return <>Add {val} as a blocklist source?</>
  return <>Remove {val}? This source will no longer be fetched.</>
}

export function App() {
  const [sources, setSources] = useState<Source[]>([])
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
      if (pending.action === "block" || pending.action === "unblock") {
        res = await fetch("/update_blocklist", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: pending.action === "block" ? "add" : "remove",
            domain: pending.value,
          }),
        })
      } else {
        res = await fetch("/sources", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            url: pending.value,
            action: pending.action === "source-add" ? "add" : "remove",
          }),
        })
      }

      if (res.ok) {
        if (pending.action === "source-add") {
          setSources((prev) => [...prev, { url: pending.value, enabled: true }])
          setNewSourceUrl("")
          toast.success("Source added")
        } else if (pending.action === "source-remove") {
          setSources((prev) => prev.filter((s) => s.url !== pending.value))
          toast.success("Source removed")
        } else if (pending.action === "block") {
          setBlockDomain("")
          toast.success(`${pending.value} blocked`)
        } else {
          setUnblockDomain("")
          toast.success(`${pending.value} unblocked`)
        }
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
    <div className="flex min-h-svh flex-col items-center p-6 pt-12">
      <div className="flex w-full max-w-3xl flex-col gap-6">

        <div className="flex gap-4">
          <Field className="flex-1">
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
          <Field className="w-44 shrink-0">
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
          <Field className="w-44 shrink-0">
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
        </div>

        <div className="rounded-md border">
          {sources.length === 0 && (
            <p className="px-3 py-4 text-center text-xs text-muted-foreground">No sources configured.</p>
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
                <Button
                  size="icon-sm"
                  variant="ghost"
                  onClick={() => openConfirm("source-remove", src.url)}
                >
                  <Trash2 />
                </Button>
              </div>
            </div>
          ))}
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
              variant={pending?.action === "source-remove" || pending?.action === "unblock" ? "destructive" : "default"}
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
