import type { Artifact, ChatMessage } from '@/types'

export interface ArtifactTurnGroup {
  key: string
  turnNumber: number
  title: string
  timeLabel: string
  artifacts: Artifact[]
}

function toMillis(value?: string | null) {
  if (!value) return Number.NaN
  const time = new Date(value).getTime()
  return Number.isFinite(time) ? time : Number.NaN
}

function formatTurnTime(value?: string | null) {
  const time = toMillis(value)
  if (!Number.isFinite(time)) return '时间待定'
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(time))
}

export function groupArtifactsByTurn(artifacts: Artifact[], messages: ChatMessage[]): ArtifactTurnGroup[] {
  if (artifacts.length === 0) return []

  const userTurns = messages
    .filter((message) => message.role === 'user')
    .map((message, index) => ({
      turnNumber: index + 1,
      timestamp: message.timestamp,
      timeMs: toMillis(message.timestamp),
    }))

  if (userTurns.length === 0) {
    return [{
      key: 'turn-1',
      turnNumber: 1,
      title: '第 1 轮',
      timeLabel: formatTurnTime(artifacts[0]?.created_at),
      artifacts,
    }]
  }

  const buckets = new Map<number, Artifact[]>()

  for (const artifact of artifacts) {
    const artifactTime = toMillis(artifact.created_at)
    let matchedTurn = userTurns[0]

    if (Number.isFinite(artifactTime)) {
      for (const turn of userTurns) {
        if (!Number.isFinite(turn.timeMs)) continue
        if (turn.timeMs <= artifactTime) {
          matchedTurn = turn
        } else {
          break
        }
      }
    } else {
      matchedTurn = userTurns[userTurns.length - 1]
    }

    const current = buckets.get(matchedTurn.turnNumber) || []
    current.push(artifact)
    buckets.set(matchedTurn.turnNumber, current)
  }

  return Array.from(buckets.entries())
    .sort((a, b) => b[0] - a[0])
    .map(([turnNumber, turnArtifacts]) => {
      const turn = userTurns.find((item) => item.turnNumber === turnNumber)
      return {
        key: `turn-${turnNumber}`,
        turnNumber,
        title: `第 ${turnNumber} 轮`,
        timeLabel: formatTurnTime(turn?.timestamp || turnArtifacts[0]?.created_at),
        artifacts: turnArtifacts,
      }
    })
}

export function findArtifactTurnGroup(artifactId: string | null, groups: ArtifactTurnGroup[]) {
  if (!artifactId) return null
  return groups.find((group) => group.artifacts.some((artifact) => artifact.id === artifactId)) || null
}
