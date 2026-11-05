/// Wire enums from the OpenAPI contract. Each carries its JSON value and a
/// tolerant parser so an unknown value from a newer API never crashes the app.
enum Role {
  admin('admin'),
  manager('manager'),
  sme('sme'),
  learner('learner');

  const Role(this.wire);
  final String wire;

  static Role parse(Object? v) =>
      values.firstWhere((e) => e.wire == v, orElse: () => Role.learner);
}

enum Channel {
  app('app'),
  whatsapp('whatsapp'),
  both('both'),
  teams('teams');

  const Channel(this.wire);
  final String wire;

  static Channel parse(Object? v) =>
      values.firstWhere((e) => e.wire == v, orElse: () => Channel.app);
}

enum DeliverySlot {
  shiftStart('shift_start'),
  breakTime('break'),
  shiftEnd('shift_end');

  const DeliverySlot(this.wire);
  final String wire;

  static DeliverySlot parse(Object? v) => values.firstWhere(
        (e) => e.wire == v,
        orElse: () => DeliverySlot.shiftStart,
      );
}

enum PlanStatus {
  open('open'),
  done('done'),
  skipped('skipped'),
  empty('empty');

  const PlanStatus(this.wire);
  final String wire;

  static PlanStatus parse(Object? v) =>
      values.firstWhere((e) => e.wire == v, orElse: () => PlanStatus.open);
}

enum PlanItemKind {
  reel('reel'),
  questionOnly('question_only');

  const PlanItemKind(this.wire);
  final String wire;

  static PlanItemKind parse(Object? v) =>
      values.firstWhere((e) => e.wire == v, orElse: () => PlanItemKind.reel);
}

enum PlanItemReason {
  isNew('new'),
  review('review');

  const PlanItemReason(this.wire);
  final String wire;

  static PlanItemReason parse(Object? v) => values.firstWhere(
        (e) => e.wire == v,
        orElse: () => PlanItemReason.isNew,
      );
}

enum PlanItemState {
  pending('pending'),
  watched('watched'),
  answered('answered');

  const PlanItemState(this.wire);
  final String wire;

  static PlanItemState parse(Object? v) => values.firstWhere(
        (e) => e.wire == v,
        orElse: () => PlanItemState.pending,
      );
}

enum ReadinessState {
  ready('ready'),
  notYet('not_yet'),
  notStarted('not_started');

  const ReadinessState(this.wire);
  final String wire;

  static ReadinessState parse(Object? v) => values.firstWhere(
        (e) => e.wire == v,
        orElse: () => ReadinessState.notStarted,
      );
}

enum OtpChannel {
  whatsapp('whatsapp'),
  sms('sms'),
  email('email');

  const OtpChannel(this.wire);
  final String wire;

  static OtpChannel parse(Object? v) =>
      values.firstWhere((e) => e.wire == v, orElse: () => OtpChannel.whatsapp);
}
