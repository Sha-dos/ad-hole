import {useEffect, useState} from "react";
import {ScrollArea} from "@/components/ui/scroll-area.tsx";
import {Separator} from "@/components/ui/separator.tsx";
import React from "react";

interface Blocklist {
  update_freq: number,
  last_updated: number,
  domains: string[],
}

export function App() {
  const [blocklist, setBlocklist] = useState<Blocklist | null>();

  useEffect(() => {
    const fetchBlocklist = async () => {
      console.log("Fetching");
      const resp = await fetch("/blocklist");
      const json = await resp.json();

      console.log(json);
      setBlocklist(json);
    }

    // should really be using websockets
    const interval = setInterval(fetchBlocklist, 500);

    return () => {
      clearInterval(interval);
    };

  }, [blocklist]);

  return (
    <div className="flex min-h-svh p-6">
      <div className="flex max-w-md min-w-0 flex-col gap-4 text-sm leading-loose">
        <ScrollArea className="h-72 w-48 rounded-md border">
          <div className="p-4">
            <h4 className="mb-4 text-sm leading-none font-medium">Blocked Domains</h4>
            {blocklist ? (blocklist.domains.map((tag) => (
              <React.Fragment key={tag}>
                <div className="text-sm">{tag}</div>
                <Separator className="my-2" />
              </React.Fragment>
            ))) : <div className="text-sm">No domains</div>}
          </div>
        </ScrollArea>
      </div>
    </div>
  )
}

export default App
