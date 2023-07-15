#include "cxx-qt-gen/ffi.cxxqt.h"

namespace cxx_qt::my_object {

MyObject::~MyObject() {}

MyObjectRust const&
MyObject::unsafeRust() const
{
  return *m_rustObj;
}

MyObjectRust&
MyObject::unsafeRustMut()
{
  return *m_rustObj;
}

void
MyObject::invokable()
{
  const ::std::lock_guard<::std::recursive_mutex> guard(*m_rustObjMutex);
  invokableWrapper();
}

::QMetaObject::Connection
MyObject::readyConnect(::rust::Fn<void(MyObject&)> func,
                       ::Qt::ConnectionType type)
{
  return ::QObject::connect(
    this,
    &MyObject::ready,
    this,
    [&, func = ::std::move(func)]() {
      const ::std::lock_guard<::std::recursive_mutex> guard(*m_rustObjMutex);
      func(*this);
    },
    type);
}

::QMetaObject::Connection
MyObject::dataChangedConnect(::rust::Fn<void(MyObject&,
                                             ::std::int32_t first,
                                             ::std::unique_ptr<Opaque> second,
                                             QPoint third,
                                             QPoint const& fourth)> func,
                             ::Qt::ConnectionType type)
{
  return ::QObject::connect(
    this,
    &MyObject::dataChanged,
    this,
    [&, func = ::std::move(func)](::std::int32_t first,
                                  ::std::unique_ptr<Opaque> second,
                                  QPoint third,
                                  QPoint const& fourth) {
      const ::std::lock_guard<::std::recursive_mutex> guard(*m_rustObjMutex);
      func(*this,
           ::std::move(first),
           ::std::move(second),
           ::std::move(third),
           ::std::move(fourth));
    },
    type);
}

::QMetaObject::Connection
MyObject::newDataConnect(::rust::Fn<void(MyObject&,
                                         ::std::int32_t first,
                                         ::std::unique_ptr<Opaque> second,
                                         QPoint third,
                                         QPoint const& fourth)> func,
                         ::Qt::ConnectionType type)
{
  return ::QObject::connect(
    this,
    &MyObject::newData,
    this,
    [&, func = ::std::move(func)](::std::int32_t first,
                                  ::std::unique_ptr<Opaque> second,
                                  QPoint third,
                                  QPoint const& fourth) {
      const ::std::lock_guard<::std::recursive_mutex> guard(*m_rustObjMutex);
      func(*this,
           ::std::move(first),
           ::std::move(second),
           ::std::move(third),
           ::std::move(fourth));
    },
    type);
}

MyObject::MyObject(QObject* parent)
  : QObject(parent)
  , m_rustObj(::cxx_qt::my_object::cxx_qt_my_object::createRs())
  , m_rustObjMutex(::std::make_shared<::std::recursive_mutex>())
{
}

} // namespace cxx_qt::my_object
