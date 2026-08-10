import { NavLink } from "react-router-dom";

const items = [
  { to: "/", label: "Dashboard", icon: "▣" },
  { to: "/chat", label: "Chat", icon: "💬" },
  { to: "/agents", label: "Agents", icon: "⚙" },
  { to: "/models", label: "Models", icon: "◈" },
  { to: "/settings", label: "Settings", icon: "☰" },
];

export default function Sidebar() {
  return (
    <aside className="flex h-full w-56 flex-col border-r border-border bg-bg-soft">
      <div className="flex items-center gap-2 px-4 py-4">
        <span className="text-lg font-bold tracking-tight text-white">AXON</span>
        <span className="rounded bg-accent/20 px-1.5 py-0.5 text-[10px] font-medium text-accent">
          v0.5
        </span>
      </div>
      <nav className="flex-1 space-y-1 px-2">
        {items.map((it) => (
          <NavLink
            key={it.to}
            to={it.to}
            end={it.to === "/"}
            className={({ isActive }) =>
              `flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors ${
                isActive
                  ? "bg-bg-hover text-white"
                  : "text-slate-400 hover:bg-bg-hover/60 hover:text-slate-200"
              }`
            }
          >
            <span className="w-4 text-center text-xs opacity-80">{it.icon}</span>
            {it.label}
          </NavLink>
        ))}
      </nav>
      <div className="border-t border-border px-4 py-3 text-[11px] text-slate-500">
        mobile multi-agent gateway
      </div>
    </aside>
  );
}
