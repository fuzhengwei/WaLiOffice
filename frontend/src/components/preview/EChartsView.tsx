import { useEffect, useMemo, useRef, type CSSProperties } from 'react'
import * as echarts from 'echarts/core'
import { BarChart, LineChart, PieChart, ScatterChart, RadarChart, GaugeChart, FunnelChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent, TitleComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import type { EChartsOption } from 'echarts'

echarts.use([
  BarChart, LineChart, PieChart, ScatterChart, RadarChart, GaugeChart, FunnelChart,
  GridComponent, TooltipComponent, LegendComponent, TitleComponent,
  CanvasRenderer,
])

type ChartType = 'line' | 'bar' | 'pie' | 'scatter' | 'radar' | 'gauge' | 'funnel'

interface EChartsViewProps {
  option?: EChartsOption
  chartType?: string
  chartData?: any
  title?: string
  className?: string
  style?: CSSProperties
}

function normalizeChartType(type?: string): ChartType {
  const lower = String(type || '').toLowerCase()
  if (['line', 'bar', 'pie', 'scatter', 'radar', 'gauge', 'funnel'].includes(lower)) return lower as ChartType
  return 'bar'
}

function toNumber(value: any) {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    const normalized = value.replace('%', '').replace(/,/g, '')
    const parsed = Number(normalized)
    if (Number.isFinite(parsed)) return parsed
  }
  return 0
}

function buildSeriesData(data: any) {
  if (Array.isArray(data?.values)) return data.values.map(toNumber)
  if (Array.isArray(data?.data)) {
    return data.data.map((item: any) => typeof item === 'object' ? toNumber(item.value) : toNumber(item))
  }
  if (Array.isArray(data?.rows)) {
    return data.rows.map((row: any[]) => toNumber(row?.[1]))
  }
  return []
}

function buildCategoryData(data: any, count: number) {
  if (Array.isArray(data?.categories)) return data.categories.map(String)
  if (Array.isArray(data?.labels)) return data.labels.map(String)
  if (Array.isArray(data?.data)) {
    const labels = data.data.map((item: any, index: number) => typeof item === 'object' ? (item.name ?? item.label ?? `数据${index + 1}`) : `数据${index + 1}`)
    if (labels.length) return labels
  }
  if (Array.isArray(data?.rows)) {
    const labels = data.rows.map((row: any[], index: number) => row?.[0] ?? `数据${index + 1}`)
    if (labels.length) return labels.map(String)
  }
  return Array.from({ length: count }, (_, index) => `数据${index + 1}`)
}

function buildOption(chartType?: string, chartData?: any, title?: string): EChartsOption {
  const type = normalizeChartType(chartType || chartData?.type)
  const data = chartData || {}
  const values = buildSeriesData(data)
  const categories = buildCategoryData(data, values.length)
  const seriesName = data.seriesName || data.name || title || '数据'

  if (type === 'pie') {
    const pieData = categories.map((name, index) => ({ name, value: values[index] ?? 0 }))
    return {
      color: ['#2563eb', '#06b6d4', '#22c55e', '#f59e0b', '#ef4444', '#8b5cf6'],
      tooltip: { trigger: 'item' },
      legend: { bottom: 0, type: 'scroll' },
      series: [{ name: seriesName, type: 'pie', radius: ['42%', '68%'], center: ['50%', '45%'], data: pieData }],
    }
  }

  if (type === 'gauge') {
    return {
      series: [{ type: 'gauge', progress: { show: true }, detail: { valueAnimation: true, formatter: '{value}%' }, data: [{ value: values[0] ?? 0, name: seriesName }] }],
    }
  }

  return {
    color: ['#2563eb', '#06b6d4', '#22c55e', '#f59e0b'],
    tooltip: { trigger: 'axis' },
    grid: { left: 36, right: 20, top: 28, bottom: 36, containLabel: true },
    xAxis: { type: 'category', data: categories, axisLabel: { color: '#64748b' } },
    yAxis: { type: 'value', axisLabel: { color: '#64748b' }, splitLine: { lineStyle: { color: '#e5e7eb' } } },
    series: [{ name: seriesName, type, smooth: type === 'line', data: values }],
  }
}

export function EChartsView({ option, chartType, chartData, title, className, style }: EChartsViewProps) {
  const ref = useRef<HTMLDivElement>(null)
  const chartRef = useRef<echarts.ECharts | null>(null)
  const resolvedOption = useMemo(() => option || buildOption(chartType, chartData, title), [option, chartType, chartData, title])

  useEffect(() => {
    const node = ref.current
    if (!node) return

    const chart = echarts.init(node, undefined, { renderer: 'canvas' })
    chartRef.current = chart
    chart.setOption(resolvedOption, true)

    const resize = () => chart.resize()
    const observer = new ResizeObserver(resize)
    observer.observe(node)
    window.addEventListener('resize', resize)

    return () => {
      observer.disconnect()
      window.removeEventListener('resize', resize)
      chart.dispose()
      chartRef.current = null
    }
  }, [])

  useEffect(() => {
    chartRef.current?.setOption(resolvedOption, true)
    window.setTimeout(() => chartRef.current?.resize(), 0)
  }, [resolvedOption])

  return <div ref={ref} className={className} style={{ width: '100%', height: '100%', minHeight: 240, ...style }} />
}
