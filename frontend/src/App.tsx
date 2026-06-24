import {Field, FieldLabel} from "@/components/ui/field.tsx";
import {Input} from "@/components/ui/input.tsx";
import {Button} from "@/components/ui/button.tsx";
import {useState} from "react";

export function App() {
  const [addDomain, setAddDomain] = useState<string>("")
  const [removeDomain, setRemoveDomain] = useState<string>("")

  const handleAction = async (action: string) => {
    try {
      // @ts-ignore
      const response = await fetch("/update_blocklist", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ domain: action == "add" ? addDomain : removeDomain, action: action }),
      })
    } catch (e) {
      console.error("Error adding domain:", e)
    }
  }

  return (
    <div className="flex min-h-svh p-6 items-center justify-center gap-6">
      <Field>
        <FieldLabel>Add a domain</FieldLabel>
        <div className="flex gap-2">
          <Input onChange={(e) => setAddDomain(e.target.value)} placeholder="Domain" />
          <Button onClick={() => handleAction("add")}>add</Button>
        </div>
      </Field>

      <Field>
        <FieldLabel>Remove a domain</FieldLabel>
        <div className="flex gap-2">
          <Input onChange={(e) => setRemoveDomain(e.target.value)} placeholder="Domain" />
          <Button onClick={() => handleAction("remove")}>remove</Button>
        </div>
      </Field>
    </div>
  )
}

export default App
