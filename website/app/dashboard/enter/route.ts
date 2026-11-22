import { NextResponse } from 'next/server'
import { COOKIE, YEAR, isKey, remembered } from '../access'

// /dashboard?key=… comes here: the key checked, the browser remembered, and
// on to /dashboard with nothing in the address. A wrong key is not found.
export async function GET(request: Request) {
  const url = new URL(request.url)
  const key = url.searchParams.get('key') ?? undefined
  const fingerprint = remembered()
  if (!fingerprint || !isKey(key)) return new NextResponse('Not found', { status: 404 })
  const response = NextResponse.redirect(new URL('/dashboard', url))
  response.cookies.set(COOKIE, fingerprint, {
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
    path: '/dashboard',
    maxAge: YEAR
  })
  return response
}
