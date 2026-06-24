import { Field, FieldLabel } from "@/components/ui/field.tsx"
import { Input } from "@/components/ui/input.tsx"
import { Button } from "@/components/ui/button.tsx"
import { Dialog, DialogContent, DialogHeader, DialogFooter, DialogTitle } from "@/components/ui/dialog.tsx"
import { Loader2, CheckCircle2, XCircle } from "lucide-react"
import { useState, useEffect } from "react"

type Action = "add" | "remove" | "source"
type Status = "idle" | "loading" | "success" | "error"

interface Pending {
  action: Action
  value: string
}

const TITLES: Record<Action, string> = {
  add: "Confirm block",
  remove: "Confirm unblock",
  source: "Confirm source change",
}

function ConfirmBody({ pending }: { pending: Pending }) {
  const val = <span className="font-medium text-foreground break-all">{pending.value}</span>
  if (pending.action === "add") return <>Block {val}?</>
  if (pending.action === "remove") return <>Unblock {val}?</>
  return <>Switch blocklist source to {val}? The new list will be fetched immediately.</>
}

function SuccessBody({ pending }: { pending: Pending }) {
  const val = <span className="text-foreground">{pending.value}</span>
  if (pending.action === "add") return <>{val} blocked successfully.</>
  if (pending.action === "remove") return <>{val} unblocked successfully.</>
  return <>Source updated successfully.</>
}

export function App() {
  const [addDomain, setAddDomain] = useState("")
  const [removeDomain, setRemoveDomain] = useState("")
  const [sourceUrl, setSourceUrl] = useState("")
  const [currentSource, setCurrentSource] = useState<string | null>(null)
  const [pending, setPending] = useState<Pending | null>(null)
  const [status, setStatus] = useState<Status>("idle")

  useEffect(() => {
    fetch("/source")
      .then((r) => r.json())
      .then((d) => setCurrentSource(d.url))
      .catch(() => {})
  }, [])

  const openConfirm = (action: Action) => {
    const value =
      action === "add" ? addDomain.trim()
      : action === "remove" ? removeDomain.trim()
      : sourceUrl.trim()
    if (!value) return
    setPending({ action, value })
    setStatus("idle")
  }

  const handleConfirm = async () => {
    if (!pending) return
    setStatus("loading")
    try {
      const res =
        pending.action === "source"
          ? await fetch("/source", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ url: pending.value }),
            })
          : await fetch("/update_blocklist", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ action: pending.action, domain: pending.value }),
            })
      if (res.ok) {
        if (pending.action === "source") setCurrentSource(pending.value)
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
        <div className="flex gap-6">
          <Field className="flex-1">
            <FieldLabel>Add a domain</FieldLabel>
            <div className="flex gap-2">
              <Input
                value={addDomain}
                onChange={(e) => setAddDomain(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && openConfirm("add")}
                placeholder="example.com"
              />
              <Button onClick={() => openConfirm("add")}>add</Button>
            </div>
          </Field>

          <Field className="flex-1">
            <FieldLabel>Remove a domain</FieldLabel>
            <div className="flex gap-2">
              <Input
                value={removeDomain}
                onChange={(e) => setRemoveDomain(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && openConfirm("remove")}
                placeholder="example.com"
              />
              <Button variant="destructive" onClick={() => openConfirm("remove")}>remove</Button>
            </div>
          </Field>
        </div>

        <Field>
          <FieldLabel>Blocklist source URL</FieldLabel>
          <div className="flex gap-2">
            <Input
              value={sourceUrl}
              onChange={(e) => setSourceUrl(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && openConfirm("source")}
              placeholder="https://raw.githubusercontent.com/..."
            />
            <Button variant="outline" onClick={() => openConfirm("source")}>change</Button>
          </div>
          {currentSource && (
            <p className="text-xs text-muted-foreground truncate">
              Current: {currentSource}
            </p>
          )}
        </Field>
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
                  variant={pending.action === "remove" ? "destructive" : "default"}
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
                {pending?.action === "source" ? "Fetching blocklist…" : "Updating blocklist…"}
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
