"""Pinned skeletons and control schedules, transcribed from
formal/lean/StreamingMirror/{Instances,Controls}.lean (mechanical)."""

from model import Skel, I, R

# Instances.lean ----------------------------------------------------------

smokeChain = Skel(
    [('D', 4, [1], 0), ('D', 3, [2], 0), ('D', 2, [3], 0), ('D', 1, [], 1)],
    rootH=4, fan=2, capLevel=2)

rMix = Skel(
    [('D', 4, [1, 2], 0), ('D', 3, [3, 4], 0), ('R', 3, [], 0),
     ('D', 2, [5, 6], 0), ('R', 2, [], 0), ('D', 1, [], 2), ('R', 1, [], 0)],
    rootH=4, fan=3, capLevel=3)

comb6 = Skel(
    [('D', 6, [1], 0), ('D', 5, [2, 3], 0), ('D', 4, [4, 5], 0),
     ('D', 4, [], 0), ('D', 3, [6], 0), ('R', 3, [], 0), ('D', 2, [7], 0),
     ('D', 1, [], 2)],
    rootH=6, fan=2, capLevel=2)


def pyramid(capLevel):
    return Skel(
        [('D', 4, [1, 2], 0),
         ('D', 3, [3, 4, 5, 6], 0), ('D', 3, [7, 8, 9, 10], 0)] +
        [('D', 2, [], 0)] * 8,
        rootH=4, fan=4, capLevel=capLevel)


# Controls.lean -----------------------------------------------------------

jam = Skel(
    [('D', 4, [1], 0),           # 0: root
     ('D', 3, [2, 3, 4], 0),     # 1: A
     ('D', 2, [5, 6, 7, 8], 0),  # 2: B1
     ('D', 2, [9], 0),           # 3: B2
     ('D', 2, [10], 0),          # 4: B3
     ('D', 1, [], 1),            # 5: c1
     ('R', 1, [], 0), ('R', 1, [], 0), ('R', 1, [], 0),  # 6,7,8
     ('R', 1, [], 0), ('R', 1, [], 0)],                  # 9, 10
    rootH=4, fan=4, capLevel=1)

trap = [
    ('iopenChoose', 'wire'), ('iopenFire',),
    ('ropenRecv',),
    ('iopenChoose', 'query'), ('iopenFire',),
    ('ropenChoose', 'wire'), ('ropenFire',),
    ('ropenChoose', 'res'), ('ropenFire',),
    ('finRes',),
    ('ropenChoose', 'query'), ('ropenFire',),
    ('walkRecvWire', (I, 3)), ('walkRecvAsked', (I, 3)),
    ('walkCommit', (I, 3), ('wire', 0)), ('walkFire', (I, 3)),
    ('walkCommit', (I, 3), ('res', 0)), ('walkFire', (I, 3)),
    ('walkRecvWire', (R, 2)), ('walkRecvAsked', (R, 2)),
    ('walkCommit', (I, 3), ('query', 0)), ('walkFire', (I, 3)),
    ('walkCommit', (R, 2), ('wire', 0)), ('walkFire', (R, 2)),
    ('walkRecvWire', (I, 1)), ('walkRecvAsked', (I, 1)),
    ('walkCommit', (R, 2), ('wire', 1)), ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('res', 0)), ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('query', 0)), ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('wire', 2)),
    ('walkCommit', (I, 1), ('wire', 0)), ('walkFire', (I, 1)),
    ('walkRecvWire', (R, 0)), ('walkRecvAsked', (R, 0)),
    ('walkCommit', (R, 0), ('wire', 0)), ('walkFire', (R, 0)),
    ('walkCommit', (R, 0), ('parent',)), ('walkFire', (R, 0)),
    ('walkCommit', (I, 1), ('wire', 1)), ('walkFire', (I, 1)),
    ('walkRecvWire', (R, 0)),
    ('walkCommit', (I, 1), ('wire', 2)), ('walkFire', (I, 1)),
    ('walkCommit', (I, 1), ('res', 0)), ('walkFire', (I, 1)),
    ('walkCommit', (I, 1), ('query', 0)), ('walkFire', (I, 1)),
    ('walkCommit', (I, 1), ('wire', 3)),
    ('absorbRecvWire',), ('absorbRecvAsked',), ('absorbSend',),
    ('asmRecvRes', (I, 1)), ('asmRecvLevel', (I, 1)), ('asmSend', (I, 1)),
    ('asmRecvRes', (R, 1)), ('asmSend', (R, 1)),
    ('asmRecvRes', (R, 2)), ('asmRecvLevel', (R, 2)),
    ('asmRecvRes', (I, 3)),
    ('walkCommit', (I, 3), ('parent',)), ('walkFire', (I, 3)),
    ('walkCommit', (I, 3), ('query', 0)), ('walkFire', (I, 3)),
    ('walkCommit', (I, 3), ('query', 0)),
    ('asmRecvRes', (I, 4)),
]

pdelay = Skel(
    [('D', 4, [1], 0),                    # 0: root
     ('D', 3, [2, 3, 4], 0),              # 1: B
     ('D', 2, [], 0),                     # 2: t1
     ('D', 2, [], 0),                     # 3: t2
     ('D', 2, [5, 6, 7, 8, 9, 10], 0),    # 4: t3
     ('R', 1, [], 0), ('R', 1, [], 0), ('R', 1, [], 0),
     ('R', 1, [], 0), ('R', 1, [], 0), ('R', 1, [], 0)],
    rootH=4, fan=6, capLevel=1)

parentTrap = [
    ('iopenChoose', 'wire'), ('iopenFire',),
    ('iopenChoose', 'query'), ('iopenFire',),
    ('ropenRecv',),
    ('ropenChoose', 'wire'), ('ropenFire',),
    ('ropenChoose', 'res'), ('ropenFire',),
    ('ropenChoose', 'query'), ('ropenFire',),
    ('finRes',),
    ('walkRecvWire', (I, 3)), ('walkRecvAsked', (I, 3)),
    ('walkCommit', (I, 3), ('wire', 0)), ('walkFire', (I, 3)),
    ('walkCommit', (I, 3), ('res', 0)), ('walkFire', (I, 3)),
    ('walkCommit', (I, 3), ('query', 0)), ('walkFire', (I, 3)),
    ('walkCommit', (I, 3), ('query', 0)),
    ('walkRecvWire', (R, 2)), ('walkRecvAsked', (R, 2)),
    ('walkCommit', (R, 2), ('wire', 0)), ('walkFire', (R, 2)),
    ('walkRecvWire', (I, 1)), ('walkRecvAsked', (I, 1)),
    ('walkFire', (I, 3)),
    ('walkCommit', (I, 3), ('query', 0)),
    ('walkCommit', (I, 1), ('parent',)), ('walkFire', (I, 1)),
    ('walkCommit', (R, 2), ('res', 0)), ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('wire', 1)), ('walkFire', (R, 2)),
    ('walkRecvWire', (I, 1)), ('walkRecvAsked', (I, 1)),
    ('walkFire', (I, 3)),
    ('walkCommit', (I, 3), ('parent',)), ('walkFire', (I, 3)),
    ('walkCloseWire', (I, 3)), ('walkCloseAsked', (I, 3)),
    ('walkCommit', (I, 1), ('parent',)),
    ('walkCommit', (R, 2), ('res', 1)),
    ('asmRecvRes', (I, 2)),
    ('walkFire', (I, 1)),
    ('asmSend', (I, 2)), ('asmRecvRes', (I, 2)),
    ('asmRecvRes', (I, 3)), ('asmRecvLevel', (I, 3)),
    ('asmSend', (I, 2)),
    ('asmRecvLevel', (I, 3)),
    ('asmRecvRes', (I, 4)),
    ('asmRecvRes', (R, 2)),
    ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('wire', 2)), ('walkFire', (R, 2)),
    ('walkRecvWire', (I, 1)), ('walkRecvAsked', (I, 1)),
    ('walkCommit', (I, 1), ('wire', 0)), ('walkFire', (I, 1)),
    ('walkCommit', (I, 1), ('wire', 1)),
    ('walkCommit', (R, 2), ('res', 2)),
    ('walkRecvWire', (R, 0)),
    ('walkFire', (I, 1)),
    ('walkCommit', (I, 1), ('wire', 2)),
    ('asmSend', (R, 2)), ('asmRecvRes', (R, 2)),
    ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('query', 2)), ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('query', 2)),
    ('walkRecvAsked', (R, 0)),
    ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('query', 2)),
    ('walkCommit', (R, 0), ('parent',)), ('walkFire', (R, 0)),
    ('walkRecvWire', (R, 0)),
    ('walkFire', (I, 1)),
    ('walkCommit', (I, 1), ('wire', 3)),
    ('walkRecvAsked', (R, 0)),
    ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('query', 2)),
    ('walkCommit', (R, 0), ('parent',)),
    ('asmRecvRes', (R, 1)),
    ('walkFire', (R, 0)),
    ('walkRecvWire', (R, 0)),
    ('walkFire', (I, 1)),
    ('walkCommit', (I, 1), ('wire', 4)),
    ('walkRecvAsked', (R, 0)),
    ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('query', 2)),
    ('walkCommit', (R, 0), ('parent',)),
    ('asmSend', (R, 1)), ('asmRecvRes', (R, 1)),
    ('walkFire', (R, 0)),
    ('walkRecvWire', (R, 0)),
    ('walkFire', (I, 1)),
    ('walkCommit', (I, 1), ('wire', 5)),
    ('walkRecvAsked', (R, 0)),
    ('walkFire', (R, 2)),
    ('walkCommit', (R, 2), ('query', 2)),
    ('walkCommit', (R, 0), ('parent',)),
]
