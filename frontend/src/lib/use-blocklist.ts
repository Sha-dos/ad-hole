import { useState, useEffect } from "react"
import { toast } from "sonner"

export interface Source {
  url: string
  enabled: boolean
}

export interface Overrides {
  added: string[]
  removed: string[]
}

export type BlocklistAction =
  | "block"
  | "unblock"
  | "source-add"
  | "source-remove"
  | "override-unblock"
  | "override-unremove"

export interface Pending {
  action: BlocklistAction
  value: string
}

export function useBlocklist() {
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

  const openConfirm = (action: BlocklistAction, value: string) => {
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
          toast.success(`Block on ${pending.value} removed`)
        }
      } else {
        res = await fetch("/update_blocklist", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action: "add", domain: pending.value }),
        })
        if (res.ok) {
          setOverrides((o) => ({ ...o, removed: o.removed.filter((d) => d !== pending.value) }))
          toast.success(`Unblock on ${pending.value} removed`)
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

  return {
    sources,
    overrides,
    newSourceUrl,
    setNewSourceUrl,
    blockDomain,
    setBlockDomain,
    unblockDomain,
    setUnblockDomain,
    pending,
    setPending,
    loading,
    openConfirm,
    handleToggle,
    handleConfirm,
  }
}

export type BlocklistState = ReturnType<typeof useBlocklist>
