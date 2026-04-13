"use client";

import React, { useState, useEffect, useCallback } from "react";
import {
  Clock,
  Zap,
  CheckCircle2,
  AlertCircle,
  Globe,
  RefreshCw,
  Terminal,
  X,
  Loader2,
  History,
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";

type JobStatus =
  | "Pending"
  | "Processing"
  | { Processing: string }
  | "Completed"
  | { Failed: string };

interface Job {
  id: string;
  source_url: string;
  target_langs: string[];
  status: JobStatus;
}

const API_BASE =
  process.env.NEXT_PUBLIC_API_URL || "https://orchestrator.p.zacharie.org";

function getStatusLabel(status: JobStatus): string {
  if (status === "Pending") return "pending";
  if (status === "Completed") return "completed";
  if (typeof status === "object" && "Failed" in status) return "failed";
  if (
    status === "Processing" ||
    (typeof status === "object" && "Processing" in status)
  )
    return "processing";
  return "pending";
}

const StatusBadge = ({ status }: { status: JobStatus }) => {
  const label = getStatusLabel(status);
  const configs: Record<
    string,
    { bg: string; text: string; icon: React.ElementType }
  > = {
    pending: { bg: "bg-slate-800 border-slate-700", text: "text-slate-400", icon: Clock },
    processing: { bg: "bg-blue-950 border-blue-700", text: "text-blue-300", icon: Zap },
    completed: { bg: "bg-emerald-950 border-emerald-700", text: "text-emerald-300", icon: CheckCircle2 },
    failed: { bg: "bg-red-950 border-red-700", text: "text-red-400", icon: AlertCircle },
  };
  const cfg = configs[label] || configs.pending;
  const Icon = cfg.icon;
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-bold uppercase tracking-wider border ${cfg.bg} ${cfg.text}`}
    >
      <Icon className="w-3 h-3" />
      {label}
    </span>
  );
};

function LogDrawer({
  job,
  onClose,
}: {
  job: Job;
  onClose: () => void;
}) {
  const [logs, setLogs] = useState<string[]>([]);
  const [done, setDone] = useState(false);
  const bottomRef = React.useRef<HTMLDivElement>(null);

  useEffect(() => {
    const es = new EventSource(`${API_BASE}/api/jobs/${job.id}/logs`);
    es.addEventListener("log", (e) => {
      setLogs((prev) => [...prev, ...e.data.split("\n")]);
    });
    es.addEventListener("done", () => {
      setDone(true);
      es.close();
    });
    es.addEventListener("error", () => {
      setLogs((prev) => [...prev, "[SSE disconnected]"]);
      es.close();
    });
    return () => es.close();
  }, [job.id]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <motion.div
      initial={{ x: "100%" }}
      animate={{ x: 0 }}
      exit={{ x: "100%" }}
      transition={{ type: "spring", damping: 25 }}
      className="fixed inset-y-0 right-0 w-full max-w-2xl z-50 glass border-l border-white/10 flex flex-col"
    >
      <div className="flex items-center justify-between p-6 border-b border-white/10">
        <div>
          <h3 className="font-bold text-lg flex items-center gap-2">
            <Terminal className="w-5 h-5 text-[#8A2BE2]" />
            Logs — Job {job.id.slice(0, 8)}
          </h3>
          <p className="text-xs text-slate-400 truncate mt-1">{job.source_url}</p>
        </div>
        <div className="flex items-center gap-3">
          <StatusBadge status={job.status} />
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-white/10 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-6 font-mono text-[12px] space-y-0.5">
        {logs.length === 0 && !done && (
          <div className="flex items-center gap-2 text-slate-500 animate-pulse">
            <Loader2 className="w-4 h-4 animate-spin" />
            Waiting for logs...
          </div>
        )}
        {logs.map((line, i) => (
          <p
            key={i}
            className={
              line.includes("ERROR") || line.includes("error")
                ? "text-red-400"
                : line.includes("warn") || line.includes("WARN")
                  ? "text-amber-400"
                  : line.includes("✅") || line.includes("completed")
                    ? "text-emerald-400"
                    : "text-slate-300"
            }
          >
            {line}
          </p>
        ))}
        {done && (
          <p className="text-emerald-400 font-bold mt-2">✅ Job finished</p>
        )}
        <div ref={bottomRef} />
      </div>
    </motion.div>
  );
}

export default function HistoryPage() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedJob, setSelectedJob] = useState<Job | null>(null);
  const [filter, setFilter] = useState<string>("all");

  const fetchJobs = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE}/api/jobs`);
      if (!res.ok) throw new Error("Failed to fetch");
      const data = await res.json();
      setJobs(Array.isArray(data) ? data : data.jobs || []);
    } catch {
      setJobs([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchJobs();
  }, [fetchJobs]);

  const filtered =
    filter === "all"
      ? jobs
      : jobs.filter((j) => getStatusLabel(j.status) === filter);

  const counts = {
    all: jobs.length,
    pending: jobs.filter((j) => getStatusLabel(j.status) === "pending").length,
    processing: jobs.filter((j) => getStatusLabel(j.status) === "processing").length,
    completed: jobs.filter((j) => getStatusLabel(j.status) === "completed").length,
    failed: jobs.filter((j) => getStatusLabel(j.status) === "failed").length,
  };

  return (
    <div className="space-y-8 pb-10">
      <div className="flex items-center justify-between flex-wrap gap-4">
        <div>
          <h2 className="text-4xl font-extrabold tracking-tight">Historique</h2>
          <p className="text-slate-400 font-medium mt-1">
            Tous les jobs de la pipeline
          </p>
        </div>
        <button
          onClick={fetchJobs}
          disabled={loading}
          className="p-2.5 glass rounded-xl hover:bg-white/10 transition-colors"
          title="Refresh"
        >
          <RefreshCw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
        </button>
      </div>

      {/* Filter tabs */}
      <div className="flex gap-2 flex-wrap">
        {(["all", "processing", "completed", "failed", "pending"] as const).map(
          (f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-4 py-1.5 rounded-full text-sm font-bold capitalize transition-all border ${filter === f
                  ? "bg-[#8A2BE2]/20 border-[#8A2BE2]/60 text-[#8A2BE2]"
                  : "bg-slate-900 border-slate-700 text-slate-400 hover:border-slate-500"
                }`}
            >
              {f}{" "}
              <span className="ml-1 opacity-60">
                ({counts[f as keyof typeof counts]})
              </span>
            </button>
          )
        )}
      </div>

      {/* Job list */}
      {loading ? (
        <div className="glass rounded-2xl p-16 text-center">
          <Loader2 className="w-10 h-10 text-slate-600 mx-auto animate-spin" />
          <p className="text-slate-500 mt-4 font-medium">Chargement...</p>
        </div>
      ) : filtered.length === 0 ? (
        <div className="glass rounded-2xl p-16 text-center">
          <History className="w-12 h-12 text-slate-700 mx-auto mb-4" />
          <p className="text-slate-500 font-medium">
            Aucun job{filter !== "all" ? ` avec le statut "${filter}"` : ""}.
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          <AnimatePresence>
            {filtered.map((job, i) => (
              <motion.div
                key={job.id}
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.04 }}
                onClick={() => setSelectedJob(job)}
                className="glass p-5 rounded-2xl border border-white/5 flex items-center gap-4 cursor-pointer hover:bg-white/[0.04] transition-all"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3 flex-wrap">
                    <p className="font-semibold truncate">{job.source_url}</p>
                    <StatusBadge status={job.status} />
                  </div>
                  <div className="flex items-center gap-4 mt-1 text-xs text-slate-500">
                    <span className="flex items-center gap-1">
                      <Globe className="w-3 h-3" />
                      {job.target_langs.join(", ")}
                    </span>
                    <span className="font-mono text-[10px] text-slate-600">
                      {job.id.slice(0, 8)}
                    </span>
                  </div>
                </div>
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      )}

      <AnimatePresence>
        {selectedJob && (
          <LogDrawer job={selectedJob} onClose={() => setSelectedJob(null)} />
        )}
      </AnimatePresence>
    </div>
  );
}
