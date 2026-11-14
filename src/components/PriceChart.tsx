import ReactECharts from "echarts-for-react";
import { formatCopper } from "../money";
import type { ItemHistory } from "../types";

const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

/** Min/median price lines with quantity bars; nulls break the lines so gaps stay visible. */
export function PriceChart({ history, houseColor }: { history: ItemHistory; houseColor: string }) {
  const pts = history.points;
  const text = css("--muted");
  const line = css("--line");
  // with only a handful of snapshots a line has nothing to connect, so show the points themselves
  const sparse = pts.filter((p) => p.median !== null).length < 30;
  const option = {
    animation: false,
    grid: { left: 70, right: 50, top: 30, bottom: 30 },
    legend: { top: 0, textStyle: { color: text }, data: ["Min buyout", "Median", "AHledger min", "Quantity"] },
    tooltip: {
      trigger: "axis",
      valueFormatter: (v: number | null) => (v === null || v === undefined ? "—" : String(v)),
      formatter: (params: { seriesName: string; value: [number, number | null]; marker: string }[]) => {
        if (!params.length) return "";
        const t = new Date(params[0].value[0]).toLocaleString([], { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
        const rows = params.map((p) => `${p.marker}${p.seriesName}: ${p.seriesName === "Quantity" ? (p.value[1] ?? "—") : formatCopper(p.value[1])}`);
        return [t, ...rows].join("<br/>");
      },
    },
    xAxis: { type: "time", axisLabel: { color: text }, axisLine: { lineStyle: { color: line } } },
    yAxis: [
      { type: "value", scale: true, axisLabel: { color: text, formatter: (v: number) => formatCopper(v) }, splitLine: { lineStyle: { color: line } } },
      { type: "value", axisLabel: { color: text }, splitLine: { show: false } },
    ],
    series: [
      { name: "Min buyout", type: "line", showSymbol: sparse, connectNulls: false, data: pts.map((p) => [p.t * 1000, p.minPrice]), lineStyle: { width: 2, color: houseColor }, itemStyle: { color: houseColor } },
      { name: "Median", type: "line", showSymbol: sparse, connectNulls: false, data: pts.map((p) => [p.t * 1000, p.median]), lineStyle: { width: 1.5, color: css("--gold") }, itemStyle: { color: css("--gold") } },
      {
        name: "AHledger min",
        type: "line",
        showSymbol: false,
        connectNulls: false,
        data: history.ahledger.map(([t, v]) => [t * 1000, v]),
        lineStyle: { width: 1.5, type: "dashed", color: css("--silver") },
        itemStyle: { color: css("--silver") },
      },
      {
        name: "Quantity",
        type: "bar",
        yAxisIndex: 1,
        barMaxWidth: 6,
        data: pts.map((p) => [p.t * 1000, p.quantity]),
        itemStyle: { color: css("--panel-2") },
      },
    ],
  };
  return <ReactECharts option={option} style={{ height: 340 }} notMerge />;
}

/** Est. units sold per UTC day. */
export function SoldChart({ daily }: { daily: [string, number][] }) {
  const text = css("--muted");
  const option = {
    animation: false,
    grid: { left: 50, right: 16, top: 10, bottom: 30 },
    tooltip: { trigger: "axis" },
    xAxis: { type: "category", data: daily.map((d) => d[0].slice(5)), axisLabel: { color: text } },
    yAxis: { type: "value", axisLabel: { color: text }, splitLine: { lineStyle: { color: css("--line") } } },
    series: [{ name: "Est. sold", type: "bar", data: daily.map((d) => d[1]), itemStyle: { color: css("--silver") }, barMaxWidth: 18 }],
  };
  return <ReactECharts option={option} style={{ height: 160 }} notMerge />;
}
