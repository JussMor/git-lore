import { useState } from "react";
import { Folder, GitBranch, Terminal, List, Layers, Plus } from "lucide-react";
import "./App.css";

function App() {
  const [selectedCommit, setCommit] = useState<string | null>(null);

  return (
    <div className="flex h-screen w-screen bg-[#1e1e1e] text-[#cccccc] font-sans selection:bg-[#264f78]">
      {/* Activity Bar (Leftmost narrow strip) */}
      <div className="w-12 bg-[#252526] flex flex-col items-center py-4 space-y-4 border-r border-[#333333]">
        <div className="p-2 bg-[#37373d] text-white rounded cursor-pointer hover:bg-[#505050]">
          <Layers size={20} />
        </div>
        <div className="p-2 text-gray-400 cursor-pointer hover:text-white">
          <Folder size={20} />
        </div>
        <div className="p-2 text-gray-400 cursor-pointer hover:text-white">
          <Terminal size={20} />
        </div>
      </div>

      {/* Sidebar (Local/Remote branches, Lore state) */}
      <div className="w-64 bg-[#252526] flex flex-col border-r border-[#333333]">
        <div className="h-10 border-b border-[#333333] flex items-center px-4 font-semibold text-sm">
          LOCAL REPO & LORE
        </div>
        <div className="flex-1 overflow-y-auto pt-2 text-sm">
          {/* Section: Branches */}
          <div className="px-3 py-1 flex justify-between items-center group cursor-pointer hover:bg-[#2a2d2e]">
            <div className="flex items-center space-x-2">
              <GitBranch size={16} className="text-[#007acc]" />
              <span>main</span>
            </div>
            <span className="text-gray-500 text-xs">HEAD</span>
          </div>

          <div className="mt-4 px-3 mb-1 text-xs font-semibold text-gray-400 uppercase tracking-widest flex justify-between">
            <span>.lore files</span>
            <Plus size={14} className="cursor-pointer hover:text-white" />
          </div>
          <div className="px-3 py-1 flex items-center space-x-2 text-gray-300 cursor-pointer hover:bg-[#2a2d2e]">
            <List size={14} />
            <span>index.md</span>
          </div>
          <div className="px-3 py-1 flex items-center space-x-2 text-gray-300 cursor-pointer hover:bg-[#2a2d2e]">
            <List size={14} />
            <span>config.yaml</span>
          </div>
        </div>
      </div>

      {/* Main Area (Graph & Diff View) */}
      <div className="flex-1 flex bg-[#1e1e1e]">
        {/* Graph Pane */}
        <div className="flex-1 flex flex-col border-r border-[#333333]">
          <div className="h-10 bg-[#252526] border-b border-[#333333] flex items-center px-4 font-semibold text-sm">
            Commit Graph
          </div>
          <div className="flex-1 p-4 overflow-y-auto">
            <div className="text-gray-500 text-sm mb-4">
              Visualizing .lore atoms and git commit graphs here...
            </div>
            {/* Fake Commits */}
            {["Added lore prism integration", "init state snapshot", "First commit"].map((msg, i) => (
              <div 
                key={i} 
                className="flex items-center pl-4 py-2 hover:bg-[#2a2d2e] cursor-pointer mb-1 rounded"
                onClick={() => setCommit(msg)}
              >
                <div className="w-4 h-4 rounded-full bg-[#007acc] mr-4 border-2 border-[#1e1e1e] shadow-[0_0_0_2px] shadow-[#007acc]"></div>
                <div className="flex-1">
                  <div className="font-medium text-[#e5e5e5]">{msg}</div>
                  <div className="text-xs text-gray-500">jussmor • 2 hours ago</div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Right Pane: Commit Details */}
        <div className="w-80 flex flex-col bg-[#1e1e1e]">
          <div className="h-10 bg-[#252526] border-b border-[#333333] flex items-center px-4 font-semibold text-sm">
            Commit Details
          </div>
          <div className="flex-1 p-4 overflow-y-auto text-sm">
            {selectedCommit ? (
              <>
                <div className="text-lg font-bold mb-2 text-white">{selectedCommit}</div>
                <div className="text-gray-400 mb-4 border-b border-[#444] pb-2">
                  Author: jussmor<br/>
                  Date: Today
                </div>
                <div className="font-semibold mb-2">Lore Meta Data</div>
                <div className="bg-[#2d2d2d] p-2 rounded font-mono text-xs text-green-400">
                  + state_checksum: 0x9a8f...<br/>
                  + atoms_touched: 3
                </div>
              </>
            ) : (
              <div className="text-gray-500 italic mt-10 text-center">
                Select a commit in the graph to view details
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
