import { Field, FieldLabel } from "@/components/ui/field.tsx"
import { Input } from "@/components/ui/input.tsx"
import { Button } from "@/components/ui/button.tsx"
import { Dialog, DialogContent, DialogHeader, DialogFooter, DialogTitle } from "@/components/ui/dialog.tsx"
import { Loader2, CheckCircle2, XCircle } from "lucide-react"
import { useState } from "react"

type Status = "idle" | "loading" | "success" | "error"

interface Pending {
  action: "add" | "remove"
  domain: string
}

export function App() {
  const [addDomain, setAddDomain] = useState("")
  const [removeDomain, setRemoveDomain] = useState("")
  const [pending, setPending] = useState<Pending | null>(null)
  const [status, setStatus] = useState<Status>("idle")

  const openConfirm = (action: "add" | "remove") => {
    const domain = action === "add" ? addDomain.trim() : removeDomain.trim()
    if (!domain) return
    setPending({ action, domain })
    setStatus("idle")
  }

  const handleConfirm = async () => {
    if (!pending) return
    setStatus("loading")
    try {
      const res = await fetch("/update_blocklist", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(pending),
      })
      setStatus(res.ok ? "success" : "error")
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
    <div className="flex min-h-svh items-center justify-center gap-6 p-6">
      <Field>
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

      <Field>
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

      <Dialog open={pending !== null} onOpenChange={(open) => !open && handleClose()}>
        <DialogContent>
          {status === "idle" && (
            <>
              <DialogHeader>
                <DialogTitle>Confirm {pending?.action}</DialogTitle>
              </DialogHeader>
              <p className="text-sm text-muted-foreground">
                {pending?.action === "add" ? "Block" : "Unblock"}{" "}
                <span className="font-medium text-foreground">{pending?.domain}</span>?
              </p>
              <DialogFooter>
                <Button variant="outline" onClick={handleClose}>Cancel</Button>
                <Button
                  variant={pending?.action === "remove" ? "destructive" : "default"}
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
              <p className="text-sm text-muted-foreground">Updating blocklist…</p>
            </div>
          )}

          {status === "success" && (
            <>
              <div className="flex flex-col items-center gap-3 py-2">
                <CheckCircle2 className="size-6 text-green-500" />
                <p className="text-sm font-medium">
                  <span className="text-foreground">{pending?.domain}</span>{" "}
                  {pending?.action === "add" ? "blocked" : "unblocked"} successfully.
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
