import { Field, FieldLabel } from "@/components/ui/field.tsx"
import { Input } from "@/components/ui/input.tsx"
import { Button } from "@/components/ui/button.tsx"
import { Switch } from "@/components/ui/switch.tsx"
import { Separator } from "@/components/ui/separator.tsx"
import { Dialog, DialogContent, DialogHeader, DialogFooter, DialogTitle } from "@/components/ui/dialog.tsx"
import { Loader2, CheckCircle2, XCircle, Trash2 } from "lucide-react"
import { useState, useEffect } from "react"

interface Source {
  url: string
  enabled: boolean
}

type Action = "block" | "unblock" | "source-add" | "source-remove"
type Status = "idle" | "loading" | "success" | "error"

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

function SuccessBody({ pending }: { pending: Pending }) {
  if (pending.action === "block") return <>{pending.value} blocked.</>
  if (pending.action === "unblock") return <>{pending.value} unblocked.</>
  if (pending.action === "source-add") return <>Source added.</>
  return <>Source removed.</>
}

export function App() {
  const [sources, setSources] = useState<Source[]>([])
  const [newSourceUrl, setNewSourceUrl] = useState("")
  const [blockDomain, setBlockDomain] = useState("")
  const [unblockDomain, setUnblockDomain] = useState("")
  const [pending, setPending] = useState<Pending | null>(null)
  const [status, setStatus] = useState<Status>("idle")

  useEffect(() => {
    fetch("/sources")
      .then((r) => r.json())
      .then((d) => setSources(d.sources ?? []))
      .catch(() => {})
  }, [])

  const openConfirm = (action: Action, value: string) => {
    if (!value) return
    setPending({ action, value })
    setStatus("idle")
  }

  const handleToggle = async (url: string, enabled: boolean) => {
    setSources((prev) => prev.map((s) => (s.url === url ? { ...s, enabled } : s)))
    await fetch("/sources", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ url, action: "toggle", enabled }),
    })
  }

  const handleConfirm = async () => {
    if (!pending) return
    setStatus("loading")
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
        } else if (pending.action === "source-remove") {
          setSources((prev) => prev.filter((s) => s.url !== pending.value))
        } else if (pending.action === "block") {
          setBlockDomain("")
        } else {
          setUnblockDomain("")
        }
        setStatus("success")
      } else {
        setStatus("error")
      }
    } catch {
      setStatus("error")
    }
  }

  const handleClose = () => {
    if (status === "loading") return
    setPending(null)
    setStatus("idle")
  }

  return (
    <div className="flex min-h-svh items-center justify-center p-6">
      <div className="flex w-full max-w-xl flex-col gap-6">

        <div className="flex flex-col gap-3">
          <p className="text-sm font-semibold">Blocklist sources</p>
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
          <div className="flex gap-2">
            <Input
              value={newSourceUrl}
              onChange={(e) => setNewSourceUrl(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && openConfirm("source-add", newSourceUrl.trim())}
              placeholder="https://example.com/blocklist.txt"
            />
            <Button variant="outline" onClick={() => openConfirm("source-add", newSourceUrl.trim())}>
              Add
            </Button>
          </div>
        </div>

        <Separator />

        <div className="flex gap-6">
          <Field className="flex-1">
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
          <Field className="flex-1">
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

      </div>

      <Dialog open={pending !== null} onOpenChange={(open) => !open && handleClose()}>
        <DialogContent>
          {status === "idle" && pending && (
            <>
              <DialogHeader>
                <DialogTitle>{TITLES[pending.action]}</DialogTitle>
              </DialogHeader>
              <p className="text-sm text-muted-foreground">
                <ConfirmBody pending={pending} />
              </p>
              <DialogFooter>
                <Button variant="outline" onClick={handleClose}>Cancel</Button>
                <Button
                  variant={pending.action === "source-remove" || pending.action === "unblock" ? "destructive" : "default"}
                  onClick={handleConfirm}
                >
                  Confirm
                </Button>
              </DialogFooter>
            </>
          )}

          {status === "loading" && (
            <div className="flex flex-col items-center gap-3 py-4">
              <Loader2 className="size-6 animate-spin text-muted-foreground" />
              <p className="text-sm text-muted-foreground">
                {pending?.action === "source-add" || pending?.action === "source-remove"
                  ? "Updating sources…"
                  : "Updating blocklist…"}
              </p>
            </div>
          )}

          {status === "success" && pending && (
            <>
              <div className="flex flex-col items-center gap-3 py-2">
                <CheckCircle2 className="size-6 text-green-500" />
                <p className="text-sm font-medium">
                  <SuccessBody pending={pending} />
                </p>
              </div>
              <DialogFooter>
                <Button onClick={handleClose}>Done</Button>
              </DialogFooter>
            </>
          )}

          {status === "error" && (
            <>
              <div className="flex flex-col items-center gap-3 py-2">
                <XCircle className="size-6 text-destructive" />
                <p className="text-sm text-muted-foreground">Something went wrong. Please try again.</p>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={handleClose}>Cancel</Button>
                <Button onClick={handleConfirm}>Retry</Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}

export default App
